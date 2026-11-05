# Security Policy

## Reporting a vulnerability

Report vulnerabilities privately through GitHub Security Advisories: open the **Security** tab of this repository and choose **Report a vulnerability**. Do not open a public issue, pull request or discussion for a security report.

Please include:

- the affected instruction, account or module (for example `quote_batch`, `VaultConfig`, `sdk/src/accounting.ts`);
- the commit you tested;
- steps or a test that reproduces the issue, and the impact you expect (loss of funds, frozen funds, wrong accounting, bypassed limit).

## Scope

In scope:

- the `sprent_vault` program in `programs/sprent_vault`;
- the accounting package in `sdk/`.

Out of scope: the Manifest program, the Pyth oracle and the Solana runtime themselves (report those to their maintainers), and findings that require a compromised admin or guardian key.

## Supported versions

| Version | Supported |
|---|---|
| 0.1.x | Yes |
