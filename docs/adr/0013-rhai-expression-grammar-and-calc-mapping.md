# ADR 0013 — calc mapping on polars + rhai expression grammar

Date: 2026-09-17 · Status: accepted

`calc` ports Words' `TableReader` / strategies / `DataAnalyzer` (Python +
polars) to polars 0.44 eager/lazy. Words' expression surface for
`transform add_column` is raw `eval(expression, {"pl": pl, "col": pl.col})`
— arbitrary Python against polars expressions. Poet replaces it with a
**rhai** expression evaluated per row, per PLAN.md's global decision.

## rhai expression grammar (`add_column`)

- `col("Name")` — the current row's cell as number/string/bool/null.
  Registered natively per evaluation pass with a shared current-row map
  (rhai scope variables are not callable as functions). A missing column is
  a runtime error: `column 'Name' not found`.
- Literals: numbers (`42`, `2.5`), strings (`"a"`), booleans.
- Operators: `+ - * / %`, comparisons `== != > < >= <=`, `&& || !`.
- `if cond { a } else { b }` is allowed.
- The result must be number/string/bool (or unit → null). Any other type,
  a type mismatch (e.g. `"a" * 2`), or a mixed number/string result column
  is a `calculation_error` envelope.
- Integral results stay ints: rhai int arithmetic yields ints, so
  `col("Q1") * 2` produces an Int64 column like polars' `col('Q1') * 2`.

## Reader mapping (parity pinned, incl. quirks)

- Fresh read from disk per invocation; `--id` → `'No table with id 'x''`
  / `'Bookmark 'x' does not wrap a table'` wordings; `--index` range error
  text; default first table with `Document has no tables` when none.
- Cell coercion: strip; empty → null; case-insensitive true/false; strict
  `-?\d+` → int; strict `-?\d+\.\d+` → float; else stripped text.
- Header dedup: `""` → `col`; k-th repeat → `base_k`; generated names are
  **not** re-checked, so a literal `name_1` can collide and the later
  column's data wins (Words' dict-collapse, mirrored).
- `--range "start:end"` is **exposed on the CLI** (an addition over Words,
  whose reader accepted the parameter but never wired a flag). Semantics:
  1-based, both ends inclusive, header always kept, indices counted over
  the full row list (header = row 0), out-of-range clamped like Python
  slices. Unlike the unused Words API path, a malformed range is a
  `validation_error` (Poet validates what Words silently ignored).

## Dtype resolution and deviations from Words' crashes

Words builds polars Series from coerced lists, so polars' strict dtype
inference *crashes* on mixed columns and the crash text becomes the error
envelope. Poet resolves one dtype per column and records the deltas:

- mixed numbers/bools + text → clean `calculation_error` (same failure
  outcome as Words, readable message);
- int+float → upcast to Float64 (Words crashed);
- bool+int → Int64 with `True`→1 (Python `bool`-is-`int` semantics);
- all-null → String (inert for stats, like Words' Null dtype).

## Strategy mapping

- `stats`: numeric columns only (bools skipped); per column
  `count`(int)/`mean`/`min`/`max`/`std`/`median`, every non-count value an
  f64 (`20.0` serializes as `20.0`), `std` null below two samples (ddof=1),
  others null with no data. Full stats omit non-numeric columns; an explicit
  `--column` yields `{}` for missing/non-numeric (Words'
  `stats.get(column, {})`).
- `aggregate`: single group column; sum/mean/min/max/count/median; result
  rows `{group_key, agg_col}`; group order is **stable (first appearance)**
  — Words' polars group order was nondeterministic. Unknown function →
  `Unknown aggregation function: X`.
- `filter`: `== != > < >= <= contains startswith endswith`, case-sensitive;
  `--value` coerced by column dtype (float when it contains `.`, else int;
  string fallback surfaces the comparison error like Words). `contains` is
  a **literal substring** match — Words' `.str.contains` was regex-based,
  a polars leak users never relied on.
- `transform`: `sort`/`rename`/`drop`/`select`/`fill_null`/`add_column`
  with Words' parameter names; unknown type → `Unknown operation type: X`;
  missing columns surface polars' own messages. Sort is stable (Words' was
  unstable). `fill_null`: compatible value/dtype pairs fill; a targeted
  mismatched dtype is a `calculation_error` (Words silently cast the whole
  column), while the whole-frame form skips incompatible columns exactly
  like Words. The transform `filter` op advertised in Words' howto does not
  exist in Words' analyzer and is not ported.
- `--operations` JSON that fails to parse is a `validation_error` envelope —
  Words' typer layer let the `json.loads` escape the envelope contract
  (empty output, bare exit 1).

## Error codes

Words' calc never set an error code; Poet maps reader/resolution failures
to `calculation_error` (strategies), `not_found` (file/id/index) and
`validation_error` (range, operations JSON) per the constitution's 1:1
mapping.
