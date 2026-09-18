# Scenario: Report lifecycle (build, close, reopen, verify)

Each `poet` call is a fresh process: session state does not survive between
separate shell calls, so multi-command work is either one `&&` chain (this
scenario) or a batch script (see `quarterly-report.md`).

Build a small report in one chain — the session starts at `document new`, so
every later command auto-opens the document:

```sh
poet document new report.docx && \
  poet heading add "Field Notes" --level 1 --id notes-title && \
  poet paragraph add "Observations from the field visit." --id intro && \
  poet table add 2 2 --id visits && \
  poet table set-range --id visits --header '[["Site", "Visits"], ["Alpha", 3]]' && \
  poet paragraph add "See the table above for totals." && \
  poet document save && \
  poet document close
```

Reopen it later — `document open` restarts the session — and verify what
survived the round-trip:

```sh
poet document open report.docx && \
  poet document info && \
  poet paragraph get --id intro && \
  poet table get --id visits && \
  poet meta describe
```

Mutations continue to auto-save on the reopened document; export a Markdown
rendering when done:

```sh
poet document export report.docx md --out report.md && poet document close
```
