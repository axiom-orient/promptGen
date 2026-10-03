# Source and license provenance

`compilation`, the std-only JSON parser/writer, compiler detail/background, and
semantic edit/fidelity/refinement evidence originate in PromptGen 5.0.0
(`751eb1c66f1a929451c9dc475c9c30f8c5e235ed`), under BSD-2-Clause.

Model/tool/usage values, artifact/tool identifiers, image observations, provider
requirements and limits originate in the Apache-2.0 UpAgent source distributed
in `upagent-system`. Provider wire values also consolidate the Apache-2.0
Vergerail UpAgent bridge. Native Codex schemas remain in Vergerail with their
original provenance.

This combined crate retains both obligations: `Apache-2.0 AND BSD-2-Clause`.
`LICENSE-APACHE` and `LICENSE-BSD` reproduce the source licenses. Extracting shared
value definitions does not transfer runtime, authentication, approval, or artifact
authority to this crate.
