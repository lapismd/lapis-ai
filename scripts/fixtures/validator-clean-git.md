# AI canonical intake: clean Git transport

`validator-clean-git.json.gz` represents the existing real released Validator
0.2.0 to0.2.1 fixture in test-owned Git commits. It does not identify a production
bot revision, installed private graph, native release or registry publication.
SHA256: `1f1680bad95ea3bf46d1caf529e3e19bb8bb793c099a7ec7e2399e3c6e73e244`.

The commits were created through Jujutsu in a temporary fixture, using an
explicit fixture identity and disabled signing. Only loose public Git objects,
per-candidate indexes, the existing source input fixture, canonical fact slots
and inert note/path variants are retained. No remotes, hooks, credentials,
working-copy configuration or user profiles are captured. The test materializes
these bytes with a fixed local Git configuration and uses read-only Git commands;
CI needs no additional Jujutsu installation or network access for this fixture.

The valid empty note and five invalid candidates have clean committed working
trees. Invalid cases contain runtime release intent, malformed metadata,
more than8192 bytes, a null YAML document, or unrelated source. The normal CLI
must accept the valid candidate and reject each invalid case. Before repair,
all five invalid candidates were accepted because Git's default working-tree
diff returned no changed paths. The ancestry assertion additionally requires
the same captured parent to supply old package bytes and committed note paths.

This transport fixture complements the complete existing canonical/lock tests.
It does not substitute a synthetic no-op for a real available dependency update.
