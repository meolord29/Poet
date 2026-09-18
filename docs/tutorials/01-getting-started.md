# Tutorial: Getting Started with Poet

## 1. Install

```bash
git clone <repo-url> && cd poet
cargo install --path .
poet --version
```

## 2. Your First Document

```bash
poet document new hello.docx && \
  poet heading add "Hello" --level 1 && \
  poet paragraph add "This is my first Poet document." && \
  poet document save --path hello.docx
```

Open `hello.docx` in Word to see the result.

## 3. Bookmark IDs

Every element Poet creates gets a stable id. Give important elements an explicit `--id`:

```bash
poet document new doc.docx && \
  poet paragraph add "Introduction." --id intro && \
  poet run add " Important!" --id intro --bold && \
  poet paragraph get --id intro && \
  poet document save --path doc.docx
```

The JSON response always echoes the assigned `id` so you can reference it later (append runs, restyle, move, delete).

## 4. Tables

```bash
poet document new doc.docx && \
  poet table add 3 2 --id people && \
  poet table set-range --id people --header '[["Name","Score"],["Alice","95"],["Bob","82"]]' && \
  poet document save --path doc.docx
```

`--header` infers column types (Name → string, Score → number) and stores a schema in metadata.

## 5. Going Further

- Run `poet howto` for the full command reference.
- See [Batch Scripts](../reference/batch-scripts.md) to automate multi-step workflows.
- See the [Quarterly Report example](../examples/quarterly-report/README.md).
