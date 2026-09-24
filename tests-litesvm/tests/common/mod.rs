//! Shared LiteSVM harness driving the REAL compiled program
//! (`target/deploy/escrowl.so`). Sighashes come from the anchor-generated
//! IDL; accounts/args are hand-encoded per that IDL. Every test asserts
//! exact token balances, never just success.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use litesvm::LiteSVM;
use solana_address::Address;
use solana_clock::Clock;
use solana_hash::Hash;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signature::Signature;
use solana_signer::Signer;
use solana_system_interface::{instruction as sys_ix, program as sys_prog};
use solana_transaction::Transaction;
use std::str::FromStr;

pub const PROGRAM_ID_STR: &str = "61YPTaqaVeh4dywJEFm21jLaRhHRqeiEiG1gGNox3zwE";
pub const TOKEN_PROGRAM_ID: Address =
    solana_address::address!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const TOKEN_2022_ID: Address =
    solana_address::address!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
pub const MINT_LEN: usize = 82;
pub const ACCOUNT_LEN: usize = 165;
pub const FEE_BPS: u16 = 250;
pub const REVIEW_WINDOW: i64 = 86_400;
pub const SELLER_DEADLINE: i64 = 86_400 * 7;
pub const ARBITER_TIMEOUT: i64 = 86_400 * 30;

pub fn program_id() -> Address {
    Address::from_str(PROGRAM_ID_STR).unwrap()
}

pub fn rent_id() -> Address {
    Address::from_str("SysvarRent111111111111111111111111111111111").unwrap()
}

/// Sighash from the anchor-generated IDL (snake_case names).
pub fn sighash(name: &str) -> [u8; 8] {
    static IDL: &str = include_str!("../../../sdk/idl/escrowl.json");
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

// ---------- token helpers (hand-encoded classic SPL layouts) ----------

pub fn token_init_mint(mint: &Address, authority: &Address, decimals: u8) -> Instruction {
    // InitializeMint { decimals, mint_authority: Pubkey, freeze: COption }; [mint(w), rent(ro)]
    let mut data = vec![0u8, decimals];
    data.extend_from_slice(authority.as_ref());
    data.push(0u8);
    Instruction {
        program_id: TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*mint, false),
            AccountMeta::new_readonly(rent_id(), false),
        ],
        data,
    }
}

pub fn token_init_account(account: &Address, mint: &Address, owner: &Address) -> Instruction {
    Instruction {
        program_id: TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*account, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(*owner, false),
            AccountMeta::new_readonly(rent_id(), false),
        ],
        data: vec![1u8],
    }
}

pub fn token_mint_to(mint: &Address, to: &Address, authority: &Address, amount: u64) -> Instruction {
    let mut data = vec![7u8];
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*mint, false),
            AccountMeta::new(*to, false),
            AccountMeta::new_readonly(*authority, true),
        ],
        data,
    }
}

// ---------- PDA helpers (must match constants.rs) ----------

pub fn config_pda() -> (Address, u8) {
    Address::find_program_address(&[b"config"], &program_id())
}

pub fn escrow_pda(buyer: &Address, seller: &Address, id: u64) -> (Address, u8) {
    Address::find_program_address(
        &[b"escrow", buyer.as_ref(), seller.as_ref(), &id.to_le_bytes()],
        &program_id(),
    )
}

pub fn vault_pda(escrow: &Address) -> (Address, u8) {
    Address::find_program_address(&[b"vault", escrow.as_ref()], &program_id())
}

// ---------- environment ----------

pub struct Env {
    pub svm: LiteSVM,
    pub admin: Keypair,
    pub mint: Address,
    pub mint_authority: Keypair,
    pub treasury: Address,
    pub config: Address,
}

impl Env {
    pub fn new() -> Self {
        Self::new_with_fee(FEE_BPS)
    }

    pub fn new_with_fee(fee_bps: u16) -> Self {
        // History disabled: double-transition tests resubmit byte-identical
        // transactions, which the dedup cache would reject before they reach
        // the program. Dedup is transport-level, orthogonal to program logic.
        let mut svm = LiteSVM::new()
            .with_default_programs()
            .with_sysvars()
            .with_transaction_history(0);
        svm.add_program(
            program_id(),
            include_bytes!("../../../target/deploy/escrowl.so"),
        );
        let admin = Keypair::new();
        svm.airdrop(&admin.pubkey(), 100_000_000_000).unwrap();

        // Mint owned by a dedicated authority (not admin).
        let mint_authority = Keypair::new();
        svm.airdrop(&mint_authority.pubkey(), 10_000_000_000).unwrap();
        let mint = Keypair::new();
        let rent = svm.minimum_balance_for_rent_exemption(MINT_LEN);
        send(
            &mut svm,
            &[&admin, &mint],
            &[
                sys_ix::create_account(
                    &admin.pubkey(),
                    &mint.pubkey(),
                    rent,
                    MINT_LEN as u64,
                    &TOKEN_PROGRAM_ID,
                ),
                token_init_mint(&mint.pubkey(), &mint_authority.pubkey(), 6),
            ],
        );
        let treasury = Keypair::new().pubkey();
        let (config, _) = config_pda();
        let mut data = sighash("initialize_config").to_vec();
        data.extend_from_slice(&fee_bps.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(mint.pubkey().as_ref());
        send(
            &mut svm,
            &[&admin],
            &[Instruction {
                program_id: program_id(),
                accounts: vec![
                    AccountMeta::new(admin.pubkey(), true),
                    AccountMeta::new_readonly(treasury, false),
                    AccountMeta::new(config, false),
                    AccountMeta::new_readonly(sys_prog::ID, false),
                ],
                data,
            }],
        );
        Env {
            svm,
            admin,
            mint: mint.pubkey(),
            mint_authority,
            treasury,
            config,
        }
    }

    /// Fresh funded keypair (actor).
    pub fn actor(&mut self) -> Keypair {
        let kp = Keypair::new();
        self.svm.airdrop(&kp.pubkey(), 10_000_000_000).unwrap();
        kp
    }

    /// Token account owned by `owner`, optionally pre-funded with `amount`.
    pub fn token_account(&mut self, owner: &Address, amount: u64) -> Address {
        let acc = Keypair::new();
        let rent = self.svm.minimum_balance_for_rent_exemption(ACCOUNT_LEN);
        self.send(&[&self.admin_clone(), &acc], &[
            sys_ix::create_account(
                &self.admin.pubkey(),
                &acc.pubkey(),
                rent,
                ACCOUNT_LEN as u64,
                &TOKEN_PROGRAM_ID,
            ),
            token_init_account(&acc.pubkey(), &self.mint, owner),
        ]);
        if amount > 0 {
            let ma = self.mint_authority.pubkey();
            let mint = self.mint;
            self.send(&[&self.mint_authority_clone()], &[
                token_mint_to(&mint, &acc.pubkey(), &ma, amount),
            ]);
        }
        acc.pubkey()
    }

    pub fn admin_clone(&self) -> Keypair {
        Keypair::try_from(&self.admin.to_bytes()[..]).unwrap()
    }

    pub fn admin_pubkey(&self) -> Address {
        self.admin.pubkey()
    }

    fn mint_authority_clone(&self) -> Keypair {
        Keypair::try_from(&self.mint_authority.to_bytes()[..]).unwrap()
    }

    pub fn send(&mut self, signers: &[&Keypair], ixs: &[Instruction]) {
        send_signed(&mut self.svm, signers, ixs)
    }

    pub fn now(&self) -> i64 {
        self.svm.get_sysvar::<Clock>().unix_timestamp
    }

    pub fn warp_to(&mut self, unix_timestamp: i64) {
        let mut clock: Clock = self.svm.get_sysvar();
        clock.unix_timestamp = unix_timestamp;
        self.svm.set_sysvar(&clock);
    }

    pub fn warp_secs(&mut self, secs: i64) {
        let t = self.now();
        self.warp_to(t + secs);
    }

    /// Token balance, 0 for missing accounts (closed vault/escrow reads 0;
    /// disappearance of a FUNDED vault still breaks conservation asserts).
    pub fn token_balance(&self, ata: &Address) -> u64 {
        match self.svm.get_account(ata) {
            None => 0,
            Some(acc) if acc.data.len() < 72 => 0,
            Some(acc) => u64::from_le_bytes(acc.data[64..72].try_into().unwrap()),
        }
    }

    pub fn fetch_escrow_opt(&self, escrow: &Address) -> Option<EscrowState> {
        self.svm.get_account(escrow).and_then(|a| {
            if a.data.len() < 200 {
                None
            } else {
                Some(EscrowState::parse(&a.data))
            }
        })
    }

    pub fn fetch_escrow(&self, escrow: &Address) -> EscrowState {
        self.fetch_escrow_opt(escrow).expect("escrow account missing")
    }
}

// ---------- instruction builders (account order per IDL) ----------

fn ro(addr: Address) -> AccountMeta {
    AccountMeta::new_readonly(addr, false)
}
fn w(addr: Address) -> AccountMeta {
    AccountMeta::new(addr, false)
}
fn ws(addr: Address) -> AccountMeta {
    AccountMeta::new(addr, true)
}

fn ix(name: &str, accounts: Vec<AccountMeta>, args: Vec<u8>) -> Instruction {
    let mut data = sighash(name).to_vec();
    data.extend(args);
    Instruction {
        program_id: program_id(),
        accounts,
        data,
    }
}

fn u64v(v: u64) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}
fn uvec(v: &[u64]) -> Vec<u8> {
    let mut out = (v.len() as u32).to_le_bytes().to_vec();
    for x in v {
        out.extend_from_slice(&x.to_le_bytes());
    }
    out
}
fn opt_u16(v: Option<u16>) -> Vec<u8> {
    match v {
        None => vec![0],
        Some(x) => {
            let mut o = vec![1];
            o.extend_from_slice(&x.to_le_bytes());
            o
        }
    }
}
fn opt_addr(v: Option<Address>) -> Vec<u8> {
    match v {
        None => vec![0],
        Some(x) => {
            let mut o = vec![1];
            o.extend_from_slice(x.as_ref());
            o
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Ctx {
    pub buyer: Address,
    pub seller: Address,
    pub arbiter: Address,
    pub mint: Address,
    pub escrow: Address,
    pub vault: Address,
    pub config: Address,
    pub buyer_ata: Address,
    pub seller_ata: Address,
    pub treasury_ata: Address,
}

pub fn ix_initialize(
    config: &Address,
    admin: &Address,
    treasury: &Address,
    fee_bps: u16,
    mints: &[Address],
) -> Instruction {
    let mut args = fee_bps.to_le_bytes().to_vec();
    args.extend((mints.len() as u32).to_le_bytes());
    for m in mints {
        args.extend(m.as_ref());
    }
    ix("initialize_config", vec![
        ws(*admin), ro(*treasury), w(*config), ro(sys_prog::ID),
    ], args)
}

pub fn ix_create(c: &Ctx, id: u64, amounts: &[u64], review: i64, deadline: i64, timeout: i64) -> Instruction {
    let mut args = u64v(id);
    args.extend(uvec(amounts));
    args.extend(review.to_le_bytes());
    args.extend(deadline.to_le_bytes());
    args.extend(timeout.to_le_bytes());
    ix("create_escrow", vec![
        ws(c.buyer), ro(c.seller), ro(c.arbiter), ro(c.mint), ro(c.config),
        w(c.escrow), w(c.vault),
        ro(sys_prog::ID), ro(TOKEN_PROGRAM_ID), ro(rent_id()),
    ], args)
}

pub fn ix_fund(c: &Ctx) -> Instruction {
    ix("fund_escrow", vec![
        ws(c.buyer), w(c.escrow), w(c.vault), w(c.buyer_ata),
        ro(c.mint), ro(c.config), ro(TOKEN_PROGRAM_ID),
    ], vec![])
}

pub fn ix_submit(c: &Ctx, seller: &Address, index: u8, evidence: [u8; 32]) -> Instruction {
    let mut args = vec![index];
    args.extend(evidence);
    ix("submit_milestone", vec![ws(*seller), w(c.escrow)], args)
}

pub fn ix_approve(c: &Ctx, buyer: &Address, index: u8) -> Instruction {
    ix("approve_milestone", vec![
        ws(*buyer), w(c.escrow), w(c.vault), w(c.seller_ata), w(c.treasury_ata),
        ro(c.mint), ro(TOKEN_PROGRAM_ID),
    ], vec![index])
}

pub fn ix_claim(c: &Ctx, seller: &Address, index: u8) -> Instruction {
    ix("claim_after_timeout", vec![
        ws(*seller), w(c.escrow), w(c.vault), w(c.seller_ata), w(c.treasury_ata),
        ro(c.mint), ro(TOKEN_PROGRAM_ID),
    ], vec![index])
}

pub fn ix_dispute(c: &Ctx, buyer: &Address, index: u8) -> Instruction {
    ix("raise_dispute", vec![ws(*buyer), w(c.escrow)], vec![index])
}

pub fn ix_resolve(c: &Ctx, arbiter: &Address, index: u8, s: u64, b: u64) -> Instruction {
    let mut args = vec![index];
    args.extend(u64v(s));
    args.extend(u64v(b));
    ix("resolve_dispute", vec![
        ws(*arbiter), w(c.escrow), w(c.vault), w(c.seller_ata), w(c.buyer_ata),
        w(c.treasury_ata), ro(c.mint), ro(TOKEN_PROGRAM_ID),
    ], args)
}

pub fn ix_cancel(c: &Ctx, buyer: &Address) -> Instruction {
    ix("cancel_escrow", vec![
        ws(*buyer), w(c.escrow), w(c.vault), w(c.buyer_ata),
        ro(c.mint), ro(TOKEN_PROGRAM_ID),
    ], vec![])
}

pub fn ix_close(c: &Ctx, buyer: &Address) -> Instruction {
    ix("close_escrow", vec![
        ws(*buyer), w(c.escrow), w(c.vault), ro(TOKEN_PROGRAM_ID),
    ], vec![])
}

pub fn ix_reclaim(c: &Ctx, buyer: &Address, index: u8) -> Instruction {
    ix("reclaim_stale_milestone", vec![
        ws(*buyer), w(c.escrow), w(c.vault), w(c.buyer_ata),
        ro(c.mint), ro(TOKEN_PROGRAM_ID),
    ], vec![index])
}

pub fn ix_expire(c: &Ctx, authority: &Address, index: u8) -> Instruction {
    ix("expire_dispute", vec![
        ws(*authority), ro(c.buyer), ro(c.seller), w(c.escrow), w(c.vault),
        w(c.seller_ata), w(c.buyer_ata), w(c.treasury_ata),
        ro(c.mint), ro(TOKEN_PROGRAM_ID),
    ], vec![index])
}

pub fn ix_set_paused(config: &Address, admin: &Address, paused: bool) -> Instruction {
    ix("set_paused", vec![ws(*admin), w(*config)], vec![paused as u8])
}

pub fn ix_update_config(
    config: &Address,
    admin: &Address,
    fee: Option<u16>,
    treasury: Option<Address>,
    mint: Option<Address>,
) -> Instruction {
    let mut args = opt_u16(fee);
    args.extend(opt_addr(treasury));
    args.extend(opt_addr(mint));
    ix("update_config", vec![
        ws(*admin), w(*config), ro(sys_prog::ID),
    ], args)
}

// ---------- sending + error decoding ----------

/// Sends signed by all given keypairs. Panics on transport failure; returns
/// Ok(metadata) or Err(FailedTransactionMetadata) for assertion.
pub fn send_signed(
    svm: &mut LiteSVM,
    signers: &[&Keypair],
    ixs: &[Instruction],
) {
    send_raw(svm, signers, ixs).unwrap_or_else(|e| panic!("tx failed: {e:?}"));
}

pub fn send_raw(
    svm: &mut LiteSVM,
    signers: &[&Keypair],
    ixs: &[Instruction],
) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    let bh: Hash = svm.latest_blockhash();
    let payer = signers[0].pubkey();
    let msg = Message::new_with_blockhash(ixs, Some(&payer), &bh);
    let mut sigs = vec![
        solana_signature::Signature::default();
        msg.header.num_required_signatures as usize
    ];
    for (i, key) in msg.account_keys.iter().take(sigs.len()).enumerate() {
        let kp = signers
            .iter()
            .find(|k| k.pubkey() == *key)
            .unwrap_or_else(|| panic!("missing signer for {key}"));
        sigs[i] = kp.sign_message(&msg.serialize());
    }
    let tx = Transaction {
        signatures: sigs,
        message: msg,
    };
    svm.send_transaction(tx)
}

/// Custom program error code from a failed tx (InstructionError(0, Custom)).
pub fn err_code(e: &litesvm::types::FailedTransactionMetadata) -> Option<u32> {
    use solana_transaction_error::TransactionError;
    match &e.err {
        TransactionError::InstructionError(_, ix_err) => {
            use solana_instruction_error::InstructionError as IE;
            match ix_err {
                IE::Custom(code) => Some(*code),
                _ => None,
            }
        }
        _ => None,
    }
}

pub fn send(svm: &mut LiteSVM, signers: &[&Keypair], ixs: &[Instruction]) {
    send_signed(svm, signers, ixs)
}

// ---------- escrow state decode (manual borsh) ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ms {    pub amount: u64,
    pub status: u8,
    pub submitted_at: i64,
    pub terminal_at: i64,
    pub disputed_at: i64,
}

// MilestoneStatus discriminants: Pending 0, Submitted 1, Released 2,
// Disputed 3, Resolved 4, Refunded 5, Cancelled 6.
pub const ST_PENDING: u8 = 0;
pub const ST_SUBMITTED: u8 = 1;
pub const ST_RELEASED: u8 = 2;
pub const ST_DISPUTED: u8 = 3;
pub const ST_RESOLVED: u8 = 4;
pub const ST_REFUNDED: u8 = 5;
pub const ST_CANCELLED: u8 = 6;
// EscrowStatus: Created 0, Funded 1, Completed 2, Cancelled 3.
pub const ES_CREATED: u8 = 0;
pub const ES_FUNDED: u8 = 1;
pub const ES_COMPLETED: u8 = 2;
pub const ES_CANCELLED: u8 = 3;

#[derive(Debug, Clone)]
pub struct EscrowState {
    pub buyer: Address,
    pub seller: Address,
    pub arbiter: Address,
    pub mint: Address,
    pub vault: Address,
    pub escrow_id: u64,
    pub milestone_count: u8,
    pub milestones: Vec<Ms>,
    pub total: u64,
    pub released: u64,
    pub refunded: u64,
    pub fee_snapshot: u16,
    pub treasury_snapshot: Address,
    pub review_window: i64,
    pub seller_deadline: i64,
    pub arbiter_timeout: i64,
    pub status: u8,
    pub created_at: i64,
    pub funded_at: i64,
}

struct Cursor<'a> {
    b: &'a [u8],
    o: usize,
}
impl<'a> Cursor<'a> {
    fn new(b: &'a [u8]) -> Self {
        // skip 8-byte anchor discriminator
        Self { b, o: 8 }
    }
    fn bytes(&mut self, n: usize) -> &'a [u8] {
        let s = &self.b[self.o..self.o + n];
        self.o += n;
        s
    }
    fn u8(&mut self) -> u8 {
        self.bytes(1)[0]
    }
    fn u16(&mut self) -> u16 {
        u16::from_le_bytes(self.bytes(2).try_into().unwrap())
    }
    fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.bytes(4).try_into().unwrap())
    }
    fn u64(&mut self) -> u64 {
        u64::from_le_bytes(self.bytes(8).try_into().unwrap())
    }
    fn i64(&mut self) -> i64 {
        i64::from_le_bytes(self.bytes(8).try_into().unwrap())
    }
    fn addr(&mut self) -> Address {
        Address::new_from_array(self.bytes(32).try_into().unwrap())
    }
}

impl EscrowState {
    pub fn parse(data: &[u8]) -> Self {
        let mut c = Cursor::new(data);
        let buyer = c.addr();
        let seller = c.addr();
        let arbiter = c.addr();
        let mint = c.addr();
        let vault = c.addr();
        let escrow_id = c.u64();
        let milestone_count = c.u8();
        let n = c.u32() as usize;
        let mut milestones = Vec::with_capacity(n);
        for _ in 0..n {
            milestones.push(Ms {
                amount: c.u64(),
                status: c.u8(),
                submitted_at: c.i64(),
                terminal_at: c.i64(),
                disputed_at: c.i64(),
            });
            c.bytes(32); // evidence_hash
        }
        let total = c.u64();
        let released = c.u64();
        let refunded = c.u64();
        let fee_snapshot = c.u16();
        let treasury_snapshot = c.addr();
        let review_window = c.i64();
        let seller_deadline = c.i64();
        let arbiter_timeout = c.i64();
        let status = c.u8();
        let created_at = c.i64();
        let funded_at = c.i64();
        EscrowState {
            buyer, seller, arbiter, mint, vault, escrow_id, milestone_count,
            milestones, total, released, refunded, fee_snapshot,
            treasury_snapshot, review_window, seller_deadline, arbiter_timeout,
            status, created_at, funded_at,
        }
    }

    pub fn locked(&self) -> u64 {
        self.milestones
            .iter()
            .filter(|m| matches!(m.status, ST_PENDING | ST_SUBMITTED | ST_DISPUTED))
            .map(|m| m.amount)
            .sum()
    }

    /// Sum of milestones closed with no fund movement (Created-cancel).
    /// Ledger equation: released + refunded + locked + cancelled == total.
    pub fn cancelled(&self) -> u64 {
        self.milestones
            .iter()
            .filter(|m| m.status == ST_CANCELLED)
            .map(|m| m.amount)
            .sum()
    }
}

// ---------- conservation ----------

pub struct Pots {
    pub vault: u64,
    pub seller: u64,
    pub buyer: u64,
    pub treasury: u64,
}

impl Pots {
    pub fn read(env: &Env, c: &Ctx) -> Self {
        Pots {
            vault: env.token_balance(&c.vault),
            seller: env.token_balance(&c.seller_ata),
            buyer: env.token_balance(&c.buyer_ata),
            treasury: env.token_balance(&c.treasury_ata),
        }
    }

    pub fn sum(&self) -> u64 {
        self.vault + self.seller + self.buyer + self.treasury
    }
}
