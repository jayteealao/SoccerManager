// The words the player panel and the Squad screen share for the person: the weaker foot's
// word, the form word, the progress word and the season goal in words. One source for both,
// because the panel's module reads the column model and the columns need these words too.
// Pure: no DOM. Form and progress leave here as words only, never as their figures.

import { NOT_YET_KNOWN } from './hidden-words.js';
import { ratingText } from './scale.js';

/// The word of a weaker foot of `percent`: one-footed under 35, decent from 35, two-footed
/// from 80.
export function footWord(percent) {
  if (percent < 35) {
    return 'one-footed';
  }
  return percent < 80 ? 'decent' : 'two-footed';
}

/// The form word of the season's `form` (the mean of his last five ratings): in form from 7.2,
/// out of form at 6.3 and below, steady between; not yet known with none.
export function formWord(form) {
  if (typeof form !== 'number' || !Number.isFinite(form)) {
    return { known: false, key: null, text: NOT_YET_KNOWN, mark: null, token: null };
  }
  if (form >= 7.2) {
    return { known: true, key: 'in_form', text: 'In form', mark: '▲', token: '--good' };
  }
  if (form <= 6.3) {
    return { known: true, key: 'out_of_form', text: 'Out of form', mark: '▼', token: '--warn' };
  }
  return { known: true, key: 'steady_form', text: 'Steady form', mark: '=', token: null };
}

/// The progress word of the season's `progress` (percent of the pace of his goal): ahead from
/// 110, on track from 90, behind below; null before a match.
export function progressWord(progress) {
  if (typeof progress !== 'number' || !Number.isFinite(progress)) {
    return null;
  }
  if (progress >= 110) {
    return 'ahead';
  }
  return progress >= 90 ? 'on track' : 'behind';
}

/// The goal of `goal` (`{ kind, target }`) in words, with the count so far from `p` (the
/// player's part of the season record): `aim` and `so` as the panel writes them, `brief` the
/// aim as the Squad column writes it, and `count` what he has so far. Null for no goal.
export function goalText(goal, p) {
  const t = Number(goal?.target);
  const n = (v) => (Number.isFinite(v) ? Math.round(v) : 0);
  const has = (v) => (Number.isFinite(v) ? v : 0);
  switch (goal?.kind) {
    case 'goals':
      return counted(`Score ${n(t)} goals`, has(p?.goals), n(t));
    case 'assists':
      return counted(`Make ${n(t)} assists`, has(p?.assists), n(t));
    case 'clean_sheets':
      return counted(`Keep ${n(t)} clean sheets`, has(p?.clean_sheets), n(t));
    case 'starts':
      return counted(`Start ${n(t)} matches`, has(p?.starts), n(t));
    case 'average_rating': {
      const ratings = Array.isArray(p?.ratings) ? p.ratings : [];
      const avg = ratings.length ? ratingText(ratings.reduce((a, r) => a + r.rating, 0) / ratings.length) : null;
      const aim = `Average a rating of ${ratingText(t)}`;
      return { aim, so: avg ? `${avg} so far` : 'no rating yet', brief: aim, count: avg ?? '—' };
    }
    case 'no_own_fault_absence': {
      const reds = has(p?.red_cards);
      return {
        aim: 'Miss no match through his own fault',
        so: reds === 0 ? 'none missed' : `${reds} sent off`,
        brief: 'No absences',
        count: null,
      };
    }
    default:
      return null;
  }
}

function counted(aim, so, target) {
  return { aim, so: `${so} of ${target}`, brief: aim, count: String(so) };
}
