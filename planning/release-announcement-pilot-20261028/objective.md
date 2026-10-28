# Release-announcement pilot: formation objective

Status: discovery and organization formation authorized; source implementation awaits human review of the exact organization candidate.

The user accepted a simple prospective test of the newly deployed external manual-orchestration organization-formation workflow. The proposed implementation task is automatic skill-release chatboard announcements: after a successful deployment, publish the release commit, changed skills and reload guidance. Notification failure must remain distinguishable from deployment failure. Future implementation tests should use isolated release directories and a test board.

This phase discovers the work, selects and explains an organization, and presents its JSON candidate and readable tree. It does not implement the feature, mutate a release/test board, run implementation tests, launch a reviewer or implementation worker, commit, or deploy. The discovery supervisor stays retained and idle at the review boundary so its knowledge can carry into approved implementation.

Root owns assembly and cross-role consistency. The human owns initial candidate review for this pilot. Later small organization revisions require coordinator approval; large revisions require human approval, with classification by coordinator and all existing explicit authority boundaries preserved. A candidate or its approval does not grant authority beyond the active task.

The release script, existing board publication interface and relevant tests are discovery inputs. Repeated deployment, publication retries, repository/author attribution, external-use defaults and failure reporting should be resolved explicitly or presented as owned decisions rather than silently expanded. Internal Rust product/runtime work is outside this external tooling task.
