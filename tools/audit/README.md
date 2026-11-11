# Rust source audits

`python3 tools/structure_audit.py` checks production Rust, both benchmark targets,
and the fuzz library and entrypoints. Generated fuzz artifacts are excluded.
`python3 tools/import_audit.py` checks production import boundaries,
facade rules, grouping, ordering, aliases, and direct sibling dependencies.
Both return a nonzero exit status for findings and support `--json`.

The shared scanner preserves offsets while masking comments and literals,
handles nested comments and balanced delimiters, and reads complete item/use
declarations. It does not expand Rust macros or replace Cargo's type, cfg, and
name-resolution checks. Source branches are inspected regardless of the current
host target.

Structural checks include:

- No implementation items or inline modules in module registries, including
  declarations spanning lines or sharing a line with another item.
- Parent imports before module declarations, then reexports, with test modules
  last and cfg predicates that require test mode.
- Dedicated type owners may declare a separate, gated test child as their final
  item. This permits direct implementation children without an extra registry;
  function files and inline test implementations remain invalid.
- Separate test files, no production placeholders, and restricted visibility.
- Nonempty reasons for lint expectations, including nested cfg attributes.
- A 500-line cohesion review, excluding architecture backends.
  `--deny-oversized` promotes the size target to an error.

The architecture registry's `select_arch_kernel!` wiring DSL is recognized only
inside the architecture tree. Arbitrary registry macros are findings.

Import checks include duplicate bindings and use-tree entries, full group-prefix
ordering, wildcard imports, same-line declarations, complete use statements,
facade boundaries, and imports routed through explicit parent bindings.
Implementation files use `super::Name`; direct sibling paths are rejected even
when that child exists. Private parent bindings are valid facade plumbing. Distinct
cfg predicates retain literal feature and architecture identities. Ordering
respects rustfmt's single-path/grouped-path convention.

The generated arithmetic-threshold reexport is a narrowly scoped wildcard
exception: its constants are emitted by the build script and form that facade's
surface. Other wildcard imports remain findings.

Benchmark engine symmetry has a separate audit:

```sh
python3 tools/bench.py check --source-only
python3 -m unittest discover -s tools/audit/tests
python3 -m unittest discover -s tools/benchmark/tests
```

The fixtures include positive controls, multiline and same-line declarations,
comment/string decoys, cfg disjunctions, duplicate imports, and reason checks.
