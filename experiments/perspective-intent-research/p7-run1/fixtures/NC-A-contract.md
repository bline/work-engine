# Scratch-file hygiene contract

## Scope

The report renderer writes one temporary file per render, under its own private scratch directory, and nothing else writes there.

## Terminal cleanup

The renderer owns its temporary file for its whole lifetime. Before it returns, the renderer must delete the temporary file it created and record in its own result whether the deletion succeeded. A render whose deletion failed reports `cleanup_failed`; it is not reported as a clean render. No other component removes or inspects the file, and no independent check is required: the file is private to the renderer and has no consumer.
