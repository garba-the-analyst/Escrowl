//! Smoke: load the real program, create a mint, initialize config.
use litesvm::LiteSVM;
use solana_address::Address;
use solana_clock::Clock;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signature::Signature;
use solana_signer::Signer;
use solana_system_interface::{instruction as sys_ix, program as sys_prog};
use solana_transaction::Transaction;

mod token {
    //! Hand-encoded classic SPL Token instructions (layouts are protocol-
    //! stable). Avoids pulling the spl-token crate, whose solana-version
    //! skew breaks LiteSVM builds.
    use solana_address::Address;
    use solana_instruction::Instruction;

    pub const PROGRAM_ID: Address =
        solana_address::address!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
    pub const MINT_LEN: usize = 82;
    pub const ACCOUNT_LEN: usize = 165;

    pub fn initialize_mint(
        mint: &Address,
        authority: &Address,
        decimals: u8,
    ) -> Instruction {
        // TokenInstruction::InitializeMint { decimals, mint_authority: Pubkey,
        //   freeze_authority: COption<Pubkey> }; accounts: [mint(w), rent(ro)].
        let mut data = vec![0u8, decimals];
        data.extend_from_slice(authority.as_ref());
        data.push(0u8); // freeze_authority: None
        Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                solana_instruction::AccountMeta::new(*mint, false),
                solana_instruction::AccountMeta::new_readonly(
                    crate::solana_sysvar::rent_id(),
                    false,
                ),
            ],
            data,
        }
    }

    pub fn initialize_account(account: &Address, mint: &Address, owner: &Address) -> Instruction {
        Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                solana_instruction::AccountMeta::new(*account, false),
                solana_instruction::AccountMeta::new_readonly(*mint, false),
                solana_instruction::AccountMeta::new_readonly(*owner, false),
                solana_instruction::AccountMeta::new_readonly(
                    crate::solana_sysvar::rent_id(),
                    false,
                ),
            ],
            data: vec![1u8],
        }
    }

    #[allow(dead_code)]
    pub fn mint_to(mint: &Address, to: &Address, authority: &Address, amount: u64) -> Instruction {
        let mut data = vec![7u8];
        data.extend_from_slice(&amount.to_le_bytes());
        Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                solana_instruction::AccountMeta::new(*mint, false),
                solana_instruction::AccountMeta::new(*to, false),
                solana_instruction::AccountMeta::new_readonly(*authority, true),
            ],
            data,
        }
    }
}

mod solana_sysvar {
    use super::Address;
    use std::str::FromStr;

    pub fn rent_id() -> Address {
        Address::from_str("SysvarRent111111111111111111111111111111111").unwrap()
    }

    #[allow(dead_code)]
    pub fn coption(key: Option<&Address>, out: &mut Vec<u8>) {
        match key {
            Some(k) => {
                out.push(1);
                out.extend_from_slice(k.as_ref());
            }
            None => out.push(0),
        }
    }
}

const PROGRAM_BYTES: &[u8] = include_bytes!("../../target/deploy/escrowl.so");

fn sighash(name: &str) -> [u8; 8] {
    // Single source of truth: the anchor-generated IDL (snake_case names).
    static IDL: &str = include_str!("../../sdk/idl/escrowl.json");
    let idl: serde_json::Value = serde_json::from_str(IDL).unwrap();
    let ix = idl["instructions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == name)
        .unwrap_or_else(|| panic!("ix {name} missing from IDL"));
    let arr = ix["discriminator"].as_array().unwrap();
    let mut disc = [0u8; 8];
    for (i, v) in arr.iter().enumerate().take(8) {
        disc[i] = v.as_u64().unwrap() as u8;
    }
    disc
}

#[test]
fn smoke_load_and_init_config() {
    use std::str::FromStr;
    // Must match declare_id! in programs/escrowl/src/lib.rs (Anchor checks).
    let program_id =
        Address::from_str("61YPTaqaVeh4dywJEFm21jLaRhHRqeiEiG1gGNox3zwE").unwrap();
    let mut svm = LiteSVM::new().with_default_programs().with_sysvars();
    svm.add_program(program_id, PROGRAM_BYTES);

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    // --- SPL mint (manual: create_account + initialize_mint) ---
    let mint = Keypair::new();
    let rent = svm.minimum_balance_for_rent_exemption(token::MINT_LEN);
    let create_ix = sys_ix::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        rent,
        token::MINT_LEN as u64,
        &token::PROGRAM_ID,
    );
    let init_ix = token::initialize_mint(&mint.pubkey(), &payer.pubkey(), 6);
    send_signed(&mut svm, &[&payer, &mint], &[create_ix, init_ix]);

    // --- initialize_config(fee_bps=250, allowlist=[mint]) ---
    let (config, _) = Address::find_program_address(&[b"config"], &program_id);
    let treasury = Keypair::new().pubkey();
    let mut data = sighash("initialize_config").to_vec();
    data.extend_from_slice(&250u16.to_le_bytes());
    data.extend_from_slice(&1u32.to_le_bytes()); // vec len
    data.extend_from_slice(mint.pubkey().as_ref());
    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(treasury, false),
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(sys_prog::ID, false),
        ],
        data,
    };
    send(&mut svm, &payer, &[ix]);

    let acc = svm.get_account(&config).unwrap();
    assert!(acc.lamports > 0, "config not created");
    // init allocates max space: 8 + 32 + 32 + 2 + 1 + (4 + 4*32) + 1 = 208.
    assert_eq!(acc.data.len(), 208);
    assert_eq!(u16::from_le_bytes([acc.data[72], acc.data[73]]), 250);

    // Clock warp works (needed for timeout tests).
    let t0: Clock = svm.get_sysvar();
    let mut clock = t0.clone();
    clock.unix_timestamp += 100_000;
    svm.set_sysvar(&clock);
    let t1: Clock = svm.get_sysvar();
    assert_eq!(t1.unix_timestamp, t0.unix_timestamp + 100_000);
}

fn send(svm: &mut LiteSVM, payer: &Keypair, ixs: &[Instruction]) {
    send_signed(svm, &[payer], ixs);
}

/// Manual message signing (avoids Signers-trait version friction).
fn send_signed(svm: &mut LiteSVM, signers: &[&Keypair], ixs: &[Instruction]) {
    let bh = svm.latest_blockhash();
    let payer = signers[0].pubkey();
    let msg = Message::new_with_blockhash(ixs, Some(&payer), &bh);
    let mut sigs = vec![
        Signature::default();
        msg.header.num_required_signatures as usize
    ];
    for (i, key) in msg.account_keys.iter().take(sigs.len()).enumerate() {
        let kp = signers
            .iter()
            .find(|k| k.pubkey() == *key)
            .unwrap_or_else(|| {
                let keys: Vec<String> = msg
                    .account_keys
                    .iter()
                    .enumerate()
                    .map(|(j, k)| format!("{j}:{k}(signer={})", msg.is_signer(j)))
                    .collect();
                panic!("missing signer for {key}; keys={keys:?}")
            });
        sigs[i] = kp.sign_message(&msg.serialize());
    }
    let tx = Transaction {
        signatures: sigs,
        message: msg,
    };
    svm.send_transaction(tx)
        .unwrap_or_else(|e| panic!("tx failed: {e:?}"));
}
