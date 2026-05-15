# Teraxlyst NOTICE

This NOTICE file records the lineage of code and design patterns used by Teraxlyst.

## Source code

Teraxlyst will be forked from **terax-ai** at tag **v0.6.5** (released 2026-05-15), licensed under Apache-2.0.

- Source: https://github.com/crynta/terax-ai
- License: Apache License, Version 2.0
- Original copyright: crynta and contributors

Per Apache-2.0 §4, when source code is added to this repository (M0 onward), all original copyright notices, attribution notices, and the LICENSE/NOTICE files from terax-ai will be preserved in this repository.

## Design inspiration (no code reuse)

Several architectural patterns in Teraxlyst are inspired by **nimbalyst** (https://github.com/nimbalyst/nimbalyst), licensed under MIT (Nimbalyst Inc., 2024-2026).

Patterns adapted (not copied):

- Two-tier append-only transcript architecture (raw provider payload + canonical event)
- YAML-defined tracker system with role-based field semantics
- Per-file visual diff approval as an interaction model for agent changes
- MCP PromptForUserInput as a structured-input pattern

Patterns are not copyrightable. The credit above is given as a courtesy. No nimbalyst source code, asset, or proprietary configuration is incorporated into Teraxlyst.

## Dependencies

When code lands (M0+), this NOTICE will be expanded to list all third-party crates and npm packages with non-permissive licenses or required attribution notices.
