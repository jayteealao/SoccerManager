// The shell's glyphs, as SVG path data. Each is drawn with `fill="currentColor"`, so its
// colour comes from the text colour, which comes from a token.

/// The icon rail, top to bottom. `null` is the 5 px gap between groups.
export const RAIL = [
  { id: 'home', label: 'Home', d: 'M8 1.8 1.2 7.6h2.3V14h3.6v-4h1.8v4h3.6V7.6h2.3z' },
  { id: 'inbox', label: 'Inbox', d: 'M1.5 2.5h13v11h-13zM3 4v5h3.2l.8 1.5h2l.8-1.5H13V4z', evenodd: true },
  {
    id: 'squad',
    label: 'Squad',
    d: 'M6 2.4a2.6 2.6 0 1 1 0 5.2 2.6 2.6 0 0 1 0-5.2zM1.2 13.5c0-3.2 2.1-4.8 4.8-4.8s4.8 1.6 4.8 4.8zM11.6 3.6a2 2 0 1 1 0 4 2 2 0 0 1 0-4zM11.2 8.7c2.3 0 3.7 1.4 3.7 4.8h-2.8c0-2-.4-3.5-1.8-4.6z',
  },
  {
    id: 'staff',
    label: 'Staff',
    d: 'M8 1.8a2.8 2.8 0 1 1 0 5.6 2.8 2.8 0 0 1 0-5.6zM2.4 14.5c0-3.6 2.4-5.5 5.6-5.5s5.6 1.9 5.6 5.5z',
  },
  {
    id: 'tactics',
    label: 'Tactics',
    d: 'M2.5 1.5h11v13h-11zM4 3v10h8V3zM8 3.8a1.2 1.2 0 1 1 0 2.4 1.2 1.2 0 0 1 0-2.4zM5.6 7.8a1.2 1.2 0 1 1 0 2.4 1.2 1.2 0 0 1 0-2.4zM10.4 7.8a1.2 1.2 0 1 1 0 2.4 1.2 1.2 0 0 1 0-2.4z',
    evenodd: true,
  },
  { id: 'prep', label: 'Match preparation', d: 'M3 1.5h7l3 3v10H3zM5 7v1.2h6V7zm0 2.5v1.2h6V9.5zM5 12v1.2h4V12z', evenodd: true },
  { id: 'match', label: 'Match', d: 'M1.2 3.2h13.6v9H1.2zM2.8 4.8v5.8h10.4V4.8zM5 13.4h6v1.4H5z', evenodd: true },
  { id: 'training', label: 'Training', d: 'M8 1.2 12.4 13h2.1v1.6h-13V13h2.1z' },
  { id: 'medical', label: 'Medical', d: 'M8 14.2 2 8.4A3.5 3.5 0 0 1 8 4a3.5 3.5 0 0 1 6 4.4z' },
  null,
  { id: 'transfers', label: 'Transfers', stroke: 'M1.5 5h10.5L9.4 2.4M14.5 11H4l2.6 2.6', width: 1.8 },
  { id: 'scouting', label: 'Scouting', stroke: 'M6.8 2.5a4.3 4.3 0 1 1 0 8.6 4.3 4.3 0 0 1 0-8.6zM9.8 9.8l4.4 4.4', width: 1.9 },
  { id: 'youth', label: 'Youth', stroke: 'M8 1.5 9.9 5.4l4.3.6-3.1 3 .7 4.3L8 11.3 4.2 13.3l.7-4.3-3.1-3 4.3-.6z', width: 1.5 },
  null,
  {
    id: 'competitions',
    label: 'Competitions',
    d: 'M4 1.8h8v4.4a4 4 0 0 1-8 0zM1.4 2.8H4v1.5H2.9c0 1 .6 1.7 1.4 2l.3 1.2A3.3 3.3 0 0 1 1.4 4zm13.2 0H12v1.5h1.1c0 1-.6 1.7-1.4 2l-.3 1.2A3.3 3.3 0 0 0 14.6 4zM7 9.8h2v2.4h2.6v2.3H4.4v-2.3H7z',
  },
  { id: 'calendar', label: 'Calendar', d: 'M1.8 3h12.4v11.3H1.8zm1.6 3.6v6.1h9.2V6.6zM4.3 1.3h1.6v3.2H4.3zm5.8 0h1.6v3.2h-1.6zM5 8h2v2H5z', evenodd: true },
  {
    id: 'international',
    label: 'International',
    stroke: 'M8 1.8a6.2 6.2 0 1 1 0 12.4 6.2 6.2 0 0 1 0-12.4zM1.8 8h12.4M8 1.8c2.1 2.1 2.1 10.3 0 12.4M8 1.8c-2.1 2.1-2.1 10.3 0 12.4',
    width: 1.3,
  },
  null,
  { id: 'club', label: 'Club', d: 'M8 1.2 15 5v1.3H1V5zM2.5 7.4h2.1v5H2.5zm4.4 0h2.2v5H6.9zm4.5 0h2.1v5h-2.1zM1 13.2h14v1.5H1z' },
  {
    id: 'finances',
    label: 'Finances',
    stroke: 'M11.6 4.6C11 3.5 9.8 2.9 8 2.9c-2 0-3.4 1-3.4 2.4 0 3.4 7.1 1.6 7.1 5.1 0 1.5-1.5 2.6-3.7 2.6-1.8 0-3.2-.7-3.8-1.9M8 1v14',
    width: 1.7,
  },
  { id: 'media', label: 'Media', d: 'M6 1.8h4v7.4H6zM7.2 11.6h1.6v2.4h2.4v1.4H4.8V14h2.4z', stroke: 'M3.6 7.2c0 2.6 2 4.4 4.4 4.4s4.4-1.8 4.4-4.4', width: 1.5 },
  null,
  { id: 'analysis', label: 'Analysis', d: 'M1.2 13.8h13.6v1.2H1.2z', stroke: 'M1.5 12 5.5 7.6l3 2.6 6-6.4', width: 1.9 },
];

export const BACK = 'M11.5 2 3.8 8l7.7 6z';
export const FORWARD = 'M4.5 2l7.7 6-7.7 6z';
export const UP_DOWN = 'M4 6.2 8 2.6l4 3.6M4 9.8l4 3.6 4-3.6';
export const SEARCH = 'M6.8 2.5a4.3 4.3 0 1 1 0 8.6 4.3 4.3 0 0 1 0-8.6zM9.8 9.8l4.4 4.4';
export const GLOBE = RAIL.find((g) => g?.id === 'international').stroke;
export const CARET = 'M4 6.2 8 10l4-3.8';

// The playback row's glyphs, as the Match live board draws them.
export const REWIND = 'M14.5 3 8 8l6.5 5zM8 3 1.5 8 8 13z';
export const PLAY = 'M4 2.2 13.5 8 4 13.8z';
export const PAUSE = 'M4 2.5h3v11H4zm5 0h3v11H9z';
export const FAST_FORWARD = 'M1.5 3 8 8l-6.5 5zM8 3l6.5 5L8 13z';

// The front door's glyphs: the start options, the in-match menu and the locked career rows.
export const PLUS = 'M7 2h2v5h5v2H9v5H7V9H2V7h5z';
export const SCREEN = 'M1.5 2.5h13v8.5h-13zM3 4v5.5h10V4zM5.5 12.5h5V14h-5z';
export const SLIDERS = 'M2 3.2h7v1.6H2zm9.5 0H14v1.6h-2.5zM2 7.2h2.5v1.6H2zm5 0h7v1.6H7zM2 11.2h7v1.6H2zm9.5 0H14v1.6h-2.5zM9 2h2.5v4H9zM4.5 6h2.5v4H4.5zM9 10h2.5v4H9z';
export const DOCUMENT = 'M3 1.5h7l3 3v10H3zM5 7v1.2h6V7zm0 2.5v1.2h6V9.5zM5 12v1.2h4V12z';
export const LEAVE = 'M2 2h7v2H4v8h5v2H2zm8.5 2.5L14 8l-3.5 3.5V9H6.5V7h4z';
export const HOME = 'M8 1.8 1.2 7.6h2.3V14h3.6v-4h1.8v4h3.6V7.6h2.3z';
export const LOCK = 'M4 7V5a4 4 0 0 1 8 0v2h1v7.5H3V7zm2 0h4V5a2 2 0 0 0-4 0z';
export const MENU = 'M2 3h12v1.8H2zm0 4.1h12v1.8H2zm0 4.1h12v1.8H2z';
