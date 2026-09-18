# Reference: Batch Scripts

Batch scripts automate multi-step document workflows in a single process (since each `poet` CLI
call is otherwise a fresh process). A batch script is a JSON array of command objects.

## Format

```json
[
  {"cmd": "document", "action": "new", "path": "report.docx"},
  {"cmd": "heading", "action": "add", "text": "Title", "level": 1},
  {"cmd": "paragraph", "action": "add", "text": "Body.", "id": "intro"},
  {"cmd": "document", "action": "save", "path": "report.docx"}
]
```

Run it:

```bash
poet batch run script.json
```

## Parameter Mapping

| Key | Used in | Maps to |
|-----|---------|---------|
| `cmd` | all | category |
| `action` | all | subcommand |
| `path` | document | file path |
| `id` | paragraph/run/heading/list/table/image/toc | `--id` (bookmark) |
| `index` | paragraph/run/table/image | `--index` (positional fallback) |
| `text` | paragraph/heading/list/page | text content |
| `level` | heading/list | level |
| `ordered` | list | bullet vs numbered |
| `style` | paragraph/table | style name |
| `rows`,`cols` | table | dimensions |
| `row`,`col`,`value` | table | cell coordinates |
| `values` | table set-range/add-row/add-column | 2D / 1D array |
| `header` | table set-range | infer schema |
| `bold`,`italic`,`underline`,`font`,`size`,`color` | run | formatting |
| `width`,`height` | image | inches |
| `levels` | toc | heading levels |
| `top/bottom/left/right`,`unit` | page margins | margins |
| `orientation`,`count`,`align`,`section` | page | layout |
| `start_type` | section | break type |
| `metadata`/`schema` | meta | JSON object |

## Rules & Caveats

1. **Session is per-process.** Use `batch run` (or `&&` chaining), not separate CLI calls.
2. **`document save` uses `path`**, falling back to the path from the most recent `new`/`open`.
3. **Reference elements by `id`.** Create with `"id": "intro"`, then use `"id": "intro"` later.
4. **Batch stops on first error.** Inspect `results` to see which command failed.
5. **Co-locate the batch JSON with the output `.docx`** for reproducibility.

## Templates

```bash
poet batch template basic basic.json
poet batch template report report.json
poet batch template data_table data.json
```
