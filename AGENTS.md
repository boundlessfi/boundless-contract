Never add "Co-Authored-By" lines to commits

Comment sparingly. A comment earns its place only by stating something the
code cannot: an invariant, a security-relevant ordering, a compatibility
trap, or a non-obvious "why". Do not narrate what the code does, restate the
function name, tag lines with version or PR numbers, or add banners over
self-evident blocks. When in doubt, delete it. Match the comment density of
the surrounding file.
