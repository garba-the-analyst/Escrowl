# SDK IDL provenance

`escrowl.json` is the **actual `anchor build` output** (copied from
`target/idl/escrowl.json`), program
`61YPTaqaVeh4dywJEFm21jLaRhHRqeiEiG1gGNox3zwE`, 12 instructions.
`/tmp/gen_idl.py` was the pre-build hand-authored stand-in — its
discriminators were verified byte-identical to anchor's, then it was
superseded by the real artifact. Regenerate with `anchor build` after any
program change and confirm `git diff` shows only the intended delta.
