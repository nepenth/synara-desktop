import React, { useState } from 'react';
import { configClass, varsClass, PopOut } from 'folds';
import { darkTheme } from '../../src/colors.css';
import { GifPicker } from '../../src/app/features/room/gif/GifPicker';
import type { GifPickerConfig, GifResult } from '../../src/app/utils/gifProvider';

// The Klipy API is answered locally, so the picker's trending feed, search,
// paging and selection run without a key. Tiles load from Klipy's CDN, which
// shows its "no longer available" image for these made-up ids.
const realFetch = window.fetch.bind(window);
window.fetch = async (input, init) => {
  const url = new URL(String(input));
  if (url.hostname !== 'api.klipy.com') return realFetch(input, init);
  const feed = url.pathname.endsWith('/trending') ? 'trend' : (url.searchParams.get('q') ?? '');
  const page = Number(url.searchParams.get('page') ?? '1');
  const data = Array.from({ length: 12 }, (_, index) => {
    const id = `${feed}-${page}-${index}`;
    return {
      id,
      slug: id,
      title: id,
      file: {
        md: { gif: { url: `https://static.klipy.com/ii/${id}/md.gif`, width: 160, height: 120 } },
        sm: { webp: { url: `https://static.klipy.com/ii/${id}/sm.webp`, width: 160, height: 120 } },
      },
    };
  });
  return new Response(JSON.stringify({ result: true, data: { data, has_next: page < 3 } }));
};

// The composer's popout portals to <body>, so the theme goes there as in the app.
document.body.classList.add(configClass, varsClass, darkTheme);

const config: GifPickerConfig = { enabled: true, provider: 'klipy', apiKey: 'fixture-key' };

export function GifPickerFixture() {
  const query = new URLSearchParams(window.location.search).get('q') ?? '';
  const [sent, setSent] = useState<GifResult>();
  return (
    <main style={{ padding: 16 }}>
      {new URLSearchParams(window.location.search).has('popout') ? (
        <PopOut
          offset={16}
          alignOffset={-44}
          position="Top"
          align="End"
          anchor={{ x: 700, y: 560, width: 32, height: 32 }}
          content={<GifPicker config={config} initialQuery={query} onSelect={setSent} />}
        >
          {null}
        </PopOut>
      ) : (
        <GifPicker config={config} initialQuery={query} onSelect={setSent} />
      )}
      <p data-testid="gif-sent">{sent?.url ?? ''}</p>
    </main>
  );
}
