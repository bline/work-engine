# Exact source recovery

All six source objects used by the three candidate corpus manifests were recovered from an existing external temporary cache. Every byte digest matches the digest recorded before profile extraction. No network substitution was required.

| Candidate | Source | Bytes | SHA-256 | Rights |
|---|---|---:|---|---|
| Gelman | Objections to Bayesian Statistics | 101,561 | `f16c517ec29dc218db6f2c823b35a164a4baa3415e8d58e35fc309293f3c6287` | Public readable; reuse unconfirmed |
| Gelman | Rejoinder | 162,593 | `383df3791a7289213fb178d07cde7b275c860131a9c84deb68be9c88d4bfeadd` | Public readable; reuse unconfirmed |
| Leveson | Safety Assurance white paper | 1,615,388 | `db3aa53b01ca641c0fa564aa946df8aede11c827b10f24f793faaf91196e7aa9` | Public readable; reuse unconfirmed |
| Leveson | Oil and Gas Senate testimony | 118,121 | `4bfd351202f90b8b47de9ad3da1a82a37137cade78291798149c1b05395b9ee4` | Public readable; reuse unconfirmed |
| Shaw | What Makes Good Research in Software Engineering? | 349,905 | `28f782115f3bc96097bfcb07fa324345425fc721cdac2f4df2044c08fb490570` | Public readable; reuse unconfirmed |
| Shaw | Writing Good Software Engineering Research Papers | 114,882 | `6e9186d33db91a1083f0f0524ae414f7867a92737d792f67aaecba765177b078` | Public readable; reuse unconfirmed |

The three corpus-manifest files also match the corpus digests retained by the profiles:

- Gelman: `18e3db1829bdac0945622dc7999fbb74d076398643d51bec2f6042016b7b0dc3`
- Leveson: `bf7bfd3b4f38387814a9d9ea920466b97e3f6bcd4e7f7e6421f2a94a5e708541`
- Shaw: `c2d35df5253001fbb4d0e95f4932b3b60bcf31e7d9efd70ba8dfd6f89aac9bd5`

PDF text was deterministically extracted with Poppler `pdftotext` 26.01.0 using UTF-8 output. HTML visible text was extracted with Python 3.14.4 `html.parser`, excluding script, style, noscript, and SVG contents. Extracted-text digests and token counts are retained in `content-addressed-corpus-manifest.json`.

Neither source bytes nor extracted full text were added to Git. The retained record contains URLs, rights status, byte lengths, source and extracted-text hashes, algorithms, and derived match offsets only.
