# Architecture decision records

Append-only history of the non-obvious decisions. A new ADR gets the next
number; superseded records stay (with a Status update), never deleted.

| ADR | Decision |
|-----|----------|
| [0001](0001-docx-rs-engine.md) | docx-rs as the .docx engine |
| [0002](0002-branch-topology.md) | branch topology: nightly trunk, main stable |
| [0003](0003-clap-derive-cli.md) | clap-derive CLI |
| [0004](0004-bookmark-addressing.md) | bookmark addressing |
| [0005](0005-lists-via-real-numbering.md) | lists via real numbering |
| [0006](0006-id-allocation-and-addressing.md) | id allocation and addressing |
| [0007](0007-table-model-mapping.md) | table model mapping |
| [0008](0008-engine-gaps.md) | engine gaps and their bridges |
| [0009](0009-run-formatting-and-emphasize.md) | run formatting and emphasize |
| [0010](0010-style-registry.md) | style registry |
| [0011](0011-page-section-mapping.md) | page/section mapping |
| [0012](0012-meta-custom-xml-part.md) | metadata in a custom XML part |
| [0013](0013-rhai-expression-grammar-and-calc-mapping.md) | rhai grammar + calc mapping |
| [0014](0014-completions-addition.md) | shell completions (addition over Words) |
| [0015](0015-qa-enforcement-layers.md) | QA enforcement layers (build gates, contract tests, dev feature) |
| [0016](0016-automated-version-ladder.md) | automated version ladder (`cargo xtask bump`) |
| [0017](0017-skill-integration.md) | skill integration (`poet register`) |
| [0018](0018-upgrade-fetches-release.md) | `poet upgrade` fetches the published release |
| [0019](0019-uninstall-semantics.md) | `poet uninstall` semantics |
| [0020](0020-install-consent-and-banner.md) | installer consent plan + branded banner |
| [0021](0021-ruleset-bypass-actors.md) | ruleset branch protection with bypass actors |
