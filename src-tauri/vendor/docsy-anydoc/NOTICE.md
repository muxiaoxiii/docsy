# Docsy AnyDoc Fork

This directory is a maintained fork of [Firecrawl AnyDoc](https://github.com/firecrawl/anydoc), based on version `0.1.8`.

It is included in the Docsy source tree under the upstream MIT license in `LICENSE`. Docsy keeps the Office, OpenDocument, RTF, EPUB and CSV parsers, removes the PDF route because Docsy has a dedicated PDF subsystem, and adds `src/docsy.rs` as the application-specific conversion boundary. Changes to the parser should be made here and covered by Docsy conversion tests rather than hidden behind an external executable.
