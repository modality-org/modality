import { expect, describe, it } from "@jest/globals";
import SSHPem from "./SSHPem";

const SSH_PUB_PEM = `ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIKHgM4uu+tPNoJDDHCtEJpzUo2S/cLajQEKEgWhx2CtP dotcontract@localhost`;
const SSH_PRIV_PEM = `-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
QyNTUxOQAAACCh4DOLrvrTzaCQwxwrRCac1KNkv3C2o0BChIFocdgrTwAAAKDb9zjm2/c4
5gAAAAtzc2gtZWQyNTUxOQAAACCh4DOLrvrTzaCQwxwrRCac1KNkv3C2o0BChIFocdgrTw
AAAEBAr2MXxQ253RWQ0dyuxB3MxzjX5lHDFSnbYjfkHcisqKHgM4uu+tPNoJDDHCtEJpzU
o2S/cLajQEKEgWhx2CtPAAAAF2RvdGNvbnRyYWN0QGZhdWx0LmxvY2FsAQIDBAUG
-----END OPENSSH PRIVATE KEY-----`;

describe("SSHPem", () => {
  it("should work", async () => {
    const pub_pem = SSHPem.fromSSHDotPubString(SSH_PUB_PEM);
    expect(pub_pem.toSSHDotPubString()).toEqual(SSH_PUB_PEM);

    const priv_pem = SSHPem.fromSSHPrivatePemString(SSH_PRIV_PEM);
    expect(priv_pem.public_key).toEqual(pub_pem.public_key);
    expect(priv_pem.private_key.length).toBe(64);
    const priv_pem2 = priv_pem.toSSHPrivatePemString();
    expect(priv_pem2).toEqual(SSH_PRIV_PEM);
  });
});
