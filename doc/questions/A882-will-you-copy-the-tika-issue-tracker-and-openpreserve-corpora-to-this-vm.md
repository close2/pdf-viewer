Status: complete
Given: 2026-11-17, in conversation — transcribed by the round
Owes: fetch both corpora directly onto this VM into ~/scratch/corpus-cache/ — cloning openpreserve/format-corpus at 366f068cec399d0cdfd61fa473de3ab6dc858098 into ~/scratch/corpus-cache/openpreserve and fetching tika-issue-tracker (batches 1, 2, 3, 5, 6, and 4 if desired) from the Internet Archive via doc/todo/03-more-corpora.md §§29 and 32's 512 MiB HTTP Range recipe verified against the published SHA-512 digests — then re-run pdf-script's script_corpus walk (a corpus / build round)

> instead of rsyncing.  Isn't it much more efficient, to find out, where I got these files from, and let the quorra-agent retrieve them?  Please try to find out, where I got them from.
>
> yes, please

Reading: adopts option (a)'s goal (populating both directories under `~/scratch/corpus-cache/` rather than lowering the `script_corpus` floors/ceilings), and instructs `quorra` to fetch both corpora directly onto the VM from their upstream sources, whose recipes are already documented in this repository:

1. **`openpreserve` (`~/scratch/corpus-cache/openpreserve`):**
   - Full (non-sparse) Git clone of `https://github.com/openpreserve/format-corpus.git` checked out at commit `366f068cec399d0cdfd61fa473de3ab6dc858098` (see `doc/third-party-data.md`, "`openpreserve/format-corpus`").

2. **`tika-issue-tracker` (`~/scratch/corpus-cache/tika-issue-tracker`):**
   - Originally fetched on the notebook using the exact Wayback Machine 512 MiB HTTP `Range` recipe documented in **`doc/todo/03-more-corpora.md` §§29 and 32**:
     - Base URL: `http://web.archive.org/web/2021id_/https://corpora.tika.apache.org/base/packaged/pdfs/pdfs_202011/batch<N>.tgz` (resolve the `302` `Location:` capture URL once per batch, fetch 512 MiB byte ranges starting fresh on any `502`/`504` rather than `curl -C -`, concatenate, verify SHA-512, and extract into `~/scratch/corpus-cache/tika-issue-tracker/`).
     - Note: the notebook held extracted `batch1`, `batch2`, `batch3`, `batch5`, and `batch6` (`batch4` had only its `.sha512` file; fetching `batch4` as well is welcome if patience permits, though `batch1`, `2`, `3`, `5`, `6` cover all 200 held `script_corpus` files). Once extracted and verified, the `.tgz` archives may be deleted if desired to save half the disk space.
   - Published Apache SHA-512 digests (from the notebook's `batch*.tgz.sha512` files):
     - `batch1.tgz` (`1 964 007 754` bytes): `8163368737a65febcf95ecba68ba5e05fd0487b1b234a1a36f438d7e6314756e2e0433f33eb0556768e1a45948d585abaa49ade350a1de728e89fc581f5fcde1`
     - `batch2.tgz` (`4 496 841 248` bytes): `d96c5e20f4e1d86ab69feb5cd5fc0f651e28eb21ff04f4918e0ff09abbadfd842905240345d77c949d31cb307b3567ad4cf307f5c80688ee90287c69adcdb141`
     - `batch3.tgz` (`3 606 571 824` bytes): `a00bbb1e97f101db7686d73c43d3f85f4cdf226bc0ffa84462fc309202bfa7a5fd600c6e2c1a77f9244bed8f7209e548cd9a29cb681919b05a6ebf34c15cbd1b`
     - `batch4.tgz`: `19479a17b5a7171a76274ada7ddd75a065c972e77c08c0b9b3519a109d64664372bf3f3269e3623f4016af3a5f8f5f68efcd36783f5332b234fefc1a6783a9ab`
     - `batch5.tgz` (`4 205 570 320` bytes): `f5650ee02b1173139f5a8e3385aa140e2a6a68fcf0c35b8211ed62825b141de2d34e70ee865dddab36cca21e2eae841bb686729dcfa1bf6dc17b706b37b99c16`
     - `batch6.tgz` (`1 303 317 582` bytes): `bc90ca4ab204b8ea40001f9c7a0dcbedf201f5164131e688f2fa4938ba12e88adbc7183ca01de3c2f44700a1275195bc8d1916e2c956076ed140a70c70d3306c`
