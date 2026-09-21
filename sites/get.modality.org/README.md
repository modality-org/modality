# get.modality.org

Package distribution for Modality: pre-built `modal` binaries, install
script, and Cargo sparse registry.

Live: [get.modality.org](https://get.modality.org)

`get.modal.money` 301s here (path preserved) so old install commands still
work. Language how-to stays at [modality.org](https://www.modality.org).

## Install

```bash
curl -fsSL https://get.modality.org/testnet/latest/install.sh | sh
```

## Deploy

Requires AWS credentials for account `826004261096`, region `us-east-1`,
and the existing Route53 hosted zone for `modality.org`.

```bash
cd sites/get.modality.org
npm install
npm run deploy
```

This stack creates the S3 bucket, CloudFront distribution, ACM cert, and
DNS. It does **not** upload package objects (those would be pruned). After
the stack exists, copy content:

```bash
aws s3 sync s3://get.modal.money-content/ s3://get.modality.org-content/
aws s3 sync static/ s3://get.modality.org-content/
aws cloudfront create-invalidation --distribution-id E1FBO6H39OPO86 --paths "/*"
```

Later package builds upload with `scripts/packages/upload.sh` to
`s3://get.modality.org-content/` and invalidate this distribution.
