# Synara Spaces Contract

Reviewed: 2026-05-26

Space sidebar organization is stored in global Matrix account data under
`in.synara.spaces`. It preserves ordering and folder grouping for joined Matrix
spaces. It is not a desktop-only preference.

Canonical payloads may include:

```ts
type SynaraSpacesContent = {
  shortcut?: string[];
  sidebar?: Array<string | SynaraSidebarFolder>;
};

type SynaraSidebarFolder = {
  id: string;
  name?: string;
  content: string[];
};
```

`shortcut` is the legacy flat space ordering. `sidebar` is the current mixed
space/folder ordering. Folder `content` entries are Matrix space room IDs.
Readers should ignore spaces the current user cannot resolve or has not joined.

There is no separate Synara favorite-room account-data namespace in the current
runtime. Favorite-like room organization must stay Matrix/client state unless a
new shared contract is added here before iOS writes it.

Schema and fixtures:

- `docs/contracts/synara-spaces-content.schema.json`
- `docs/contracts/fixtures/synara-spaces-content.json`
