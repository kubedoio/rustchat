# Developer Certificate of Origin

RustChat uses the Developer Certificate of Origin (DCO) to certify that contributors have the right to submit their contributions.

## Signing Off

Every commit message must contain a `Signed-off-by` line that matches the commit author's name and email:

```bash
git commit -s -m "feat: add channel search endpoint"
```

The `-s` flag automatically appends:

```
Signed-off-by: Your Name <your.email@example.com>
```

## DCO Text

By contributing to RustChat, you agree to the following terms:

```
Developer Certificate of Origin
Version 1.1

Copyright (C) 2004, 2006 The Linux Foundation and its contributors.

Everyone is permitted to copy and distribute verbatim copies of this
license document, but changing it is not allowed.


Developer's Certificate of Origin 1.1

By making a contribution to this project, I certify that:

(a) The contribution was created in whole or in part by me and I
    have the right to submit it under the open source license
    indicated in the file; or

(b) The contribution is based upon previous work that, to the best
    of my knowledge, is covered under an appropriate open source
    license and I have the right under that license to submit that
    work with modifications, whether created in whole or in part
    by me, under the same open source license (unless I am
    permitted to submit under a different license), as indicated
    in the file; or

(c) The contribution was provided directly to me by some other
    person who certified (a), (b) or (c) and I have not modified
    it.

(d) I understand and agree that this project and the contribution
    are public and that a record of the contribution (including
    all personal information I submit with it, including my
    sign-off) is maintained indefinitely and may be redistributed
    consistent with this project or the open source license(s)
    involved.
```

## Fixing a Missing Sign-Off

If you forgot to sign off previous commits, you can amend them:

```bash
# For the most recent commit
git commit --amend --sign-off

# For multiple commits in a branch
git rebase --signoff main
```

After amending, force-push to your branch:

```bash
git push --force-with-lease origin your-branch
```

## Enforcement

The DCO is enforced in CI via the `.github/workflows/dco.yml` workflow. Pull requests with commits that lack a `Signed-off-by` line will fail the required DCO check.
