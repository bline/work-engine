# Disposable job runner cleanup requirement (synthetic fixture NC-C, parent)

Status: constructed negative control for P7 run 1. Not a real Work Engine contract.

## Terminal cleanup

After a job is cancelled, an independent check, run after the job's process has exited, must confirm that the job's container, identified by its full container ID, is absent from the container engine that launched it. If the container is still present, or its state cannot be established, the job is not reported as cleanly cancelled. The requirement does not say how the container is removed or by whom.
