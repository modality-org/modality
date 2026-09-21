# modality.network website

Homepage for the **Modality Network**, served at
[modality.network](https://modality.network) and
[www.modality.network](https://www.modality.network).

This replaces the `modal.money` landing as the public network site.
Language how-to stays at [modality.org](https://www.modality.org).
Package downloads stay at [get.modality.org](https://get.modality.org).

DNS TXT records for node bootstrappers (`_dnsaddr.*.modality.network`)
are managed separately by `rust/modality-networks` and are not part of
this stack.

## Architecture

CDK stack in this directory deploys:

- S3 bucket for static content (private; CloudFront origin)
- CloudFront distribution with HTTPS for apex and `www`
- ACM certificate (us-east-1)
- Route53 A/AAAA aliases

Existing Route53 hosted zone: `modality.network`.

## Deploy

Requires AWS credentials that can manage S3, CloudFront, ACM, Route53,
and CloudFormation in account `826004261096`, region `us-east-1`.

```bash
cd sites/www.modality.network
npm install
npx cdk bootstrap aws://ACCOUNT/us-east-1   # first time only
npm run deploy
```

Content updates: edit `static/`, then `npm run deploy` (uploads to S3
and invalidates CloudFront).
