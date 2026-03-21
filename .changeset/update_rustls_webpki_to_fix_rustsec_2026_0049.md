---
knope: patch
---

# Update rustls-webpki to fix RUSTSEC-2026-0049

`rustls-webpki 0.103.9` contains a security vulnerability ([RUSTSEC-2026-0049](https://rustsec.org/advisories/RUSTSEC-2026-0049) / [GHSA-pwjx-qhcg-rvj4](https://github.com/rustls/webpki/security/advisories/GHSA-pwjx-qhcg-rvj4)): CRLs are not considered authoritative by Distribution Point due to faulty matching logic.
