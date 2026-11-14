# Security policy

A keyboard sees everything its user types, including passwords and tokens, so security
reports are taken seriously.

## Reporting a vulnerability

Please **do not open a public issue**. Report privately through GitHub:
[Report a vulnerability](https://github.com/tristanseltec/super-keyboard/security/advisories/new).

Include what an attacker could do, the affected platform and version or commit, and steps
to reproduce. You should get a reply within a week. Fixes are released before details are
made public, and reporters are credited unless they prefer otherwise.

## What counts

For example: typed text, clipboard contents or key events leaving the device or being
written to disk or logs; another app or website reading or injecting keystrokes through
Super Keyboard; a layout file that can execute code; the desktop input service accepting
input from anything other than the local UI or a paired device.

## Supported versions

Super Keyboard is pre-release. Fixes go to `main` and the latest release.
