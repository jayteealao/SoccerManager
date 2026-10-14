"""Off-ball runs of forwards from tracking data, by the run definition the engine's runs sink
uses (engine `runs` module): a run is a spell of RUN_MIN_S or more at RUN_SPEED or faster by a
player of the side with the ball in play who is not the carrier. For each run of a forward it
writes the facts the shared classification reads (in behind, coming short): the runner's
depth, the second-last defender's depth and the ball's depth at the run's first and last
frame, and the runner's distance to the ball there. Depth is metres toward the goal the
runner's side attacks, from the halfway line.

Sources (folders of DATA_DIR): PFF FIFA World Cup 2022 (`pff-wc2022`, national teams), SkillCorner
A-League open data, IDSSE Bundesliga (DFL), Metrica sample games. Alfheim is not read: it
tracks one team only, with no ball and no opponents, so no defensive line or possession.

Usage: python scripts/tracking-runs.py DATA_DIR OUT.csv.gz [source ...]

Write OUT as `tracking_runs.csv.gz` in the real data folder `engine-cli signatures` reads.
"""
import bz2, csv, gzip, json, math, os, re, sys
import xml.etree.ElementTree as ET
from concurrent.futures import ProcessPoolExecutor

# The workers read the data folder from the environment the main process sets.
DATA = os.environ.get('SM_TRACKING_DATA', '')
RUN_SPEED = 4.0
RUN_MIN_S = 1.0
HZ = 10.0
PITCH = (105.0, 68.0)
COLUMNS = ['source', 'competition', 'national', 'match', 'team', 'player', 'position', 'period',
           't0', 't1', 'depth0', 'depth1', 'line0', 'line1', 'ball0', 'ball1', 'balld0', 'balld1']


class Match:
    """Frames at about 10 Hz: (t, period, in_play, poss team 0/1 or None, carrier key or None,
    ball (x, y) or None, {key: (x, y)}); roles: key -> (team, position, forward)."""

    def __init__(self, source, competition, national, match, teams):
        self.source, self.competition, self.national, self.match = source, competition, national, match
        self.teams = teams
        self.frames = []
        self.roles = {}


def runs_of(m):
    """The forwards' runs of match m, as rows."""
    out = []
    frames = m.frames
    n = len(frames)
    if n < 3:
        return out
    # Attack direction per team and period: the side whose players stand at lower x on
    # average in the period's in-play frames attacks +x.
    sums = {}
    for (t, per, live, poss, car, ball, pos) in frames:
        if not live:
            continue
        for k, (x, y) in pos.items():
            team = m.roles.get(k, (None,))[0]
            if team is None:
                continue
            s = sums.setdefault((per, team), [0.0, 0])
            s[0] += x
            s[1] += 1
    direction = {}
    for per in {p for (p, _) in sums}:
        a, b = sums.get((per, 0)), sums.get((per, 1))
        if not a or not b:
            continue
        home_low = a[0] / a[1] < b[0] / b[1]
        direction[(per, 0)] = 1.0 if home_low else -1.0
        direction[(per, 1)] = -direction[(per, 0)]
    keys = [k for k, r in m.roles.items() if r[2]]
    for k in keys:
        team = m.roles[k][0]
        # speed by central difference over +-2 frames (0.4 s)
        run = None
        for i in range(n):
            t, per, live, poss, car, ball, pos = frames[i]
            p = pos.get(k)
            speed = 0.0
            if p is not None and 2 <= i < n - 2:
                a, b = frames[i - 2], frames[i + 2]
                pa, pb = a[6].get(k), b[6].get(k)
                if pa is not None and pb is not None and a[1] == per and b[1] == per and b[0] > a[0]:
                    speed = math.hypot(pb[0] - pa[0], pb[1] - pa[1]) / (b[0] - a[0])
            ok = (p is not None and live and poss == team and car != k and speed >= RUN_SPEED
                  and (per, team) in direction)
            if ok and run is None:
                run = i
            elif not ok and run is not None:
                start, run = run, None
                if (i - start) / HZ + 1e-9 < RUN_MIN_S:
                    continue
                end = i if p is not None and frames[i][1] == frames[start][1] else i - 1
                row = facts(m, k, team, direction, frames[start], frames[end])
                if row:
                    out.append(row)
    return out


def facts(m, k, team, direction, f0, f1):
    d = direction[(f0[1], team)]
    vals = []
    for f in (f0, f1):
        pos, ball = f[6], f[5]
        p = pos.get(k)
        if p is None or ball is None:
            return None
        opp = sorted((q[0] * d for kk, q in pos.items() if m.roles.get(kk, (None,))[0] == 1 - team),
                     reverse=True)
        if len(opp) < 2:
            return None
        vals.append((p[0] * d, opp[1], ball[0] * d, math.hypot(p[0] - ball[0], p[1] - ball[1])))
    (a0, l0, b0, bd0), (a1, l1, b1, bd1) = vals
    r = m.roles[k]
    return [m.source, m.competition, int(m.national), m.match, m.teams[team], k, r[1], f0[1],
            f'{f0[0]:.2f}', f'{f1[0]:.2f}', f'{a0:.2f}', f'{a1:.2f}', f'{l0:.2f}', f'{l1:.2f}',
            f'{b0:.2f}', f'{b1:.2f}', f'{bd0:.2f}', f'{bd1:.2f}']


# ---------------------------------------------------------------- SkillCorner
SC_FORWARDS = {'CF', 'LF', 'RF', 'LW', 'RW'}


def skillcorner(mid):
    base = os.path.join(DATA, 'skillcorner', 'data', 'matches', mid)
    meta = json.load(open(os.path.join(base, f'{mid}_match.json'), encoding='utf-8'))
    home, away = meta['home_team']['id'], meta['away_team']['id']
    names = {home: meta['home_team']['short_name'], away: meta['away_team']['short_name']}
    m = Match('skillcorner_open', 'A-League', False, mid, [names[home], names[away]])
    for p in meta['players']:
        role = (p.get('player_role') or {}).get('acronym') or ''
        team = 0 if p['team_id'] == home else 1
        m.roles[p['id']] = (team, role, role in SC_FORWARDS)
    # ball in play with a team in possession: the phases of play.
    phases = []
    with open(os.path.join(base, f'{mid}_phases_of_play.csv'), encoding='utf-8') as f:
        for r in csv.DictReader(f):
            team = 0 if int(r['team_in_possession_id']) == home else 1
            phases.append((int(r['frame_start']), int(r['frame_end']), team))
    phases.sort()
    j = 0
    with open(os.path.join(base, f'{mid}_tracking_extrapolated.jsonl'), encoding='utf-8') as f:
        for line in f:
            d = json.loads(line)
            per = d.get('period')
            if per is None:
                continue
            fr = d['frame']
            while j < len(phases) and phases[j][1] < fr:
                j += 1
            live = j < len(phases) and phases[j][0] <= fr <= phases[j][1]
            poss = phases[j][2] if live else None
            b = d['ball_data']
            ball = (b['x'], b['y']) if b and b.get('x') is not None else None
            car = (d.get('possession') or {}).get('player_id')
            pos = {p['player_id']: (p['x'], p['y']) for p in d['player_data']}
            m.frames.append((fr / 10.0, per, live, poss, car, ball, pos))
    return m


# ---------------------------------------------------------------- PFF
PFF_FORWARDS = {'CF', 'LW', 'RW'}


def pff(gid):
    base = os.path.join(DATA, 'pff-wc2022')
    meta = json.load(open(os.path.join(base, 'Metadata', f'{gid}.json'), encoding='utf-8'))[0]
    roster = json.load(open(os.path.join(base, 'Rosters', f'{gid}.json'), encoding='utf-8'))
    home, away = meta['homeTeam']['id'], meta['awayTeam']['id']
    m = Match('pff_wc2022', 'FIFA World Cup 2022', True, gid,
              [meta['homeTeam']['name'], meta['awayTeam']['name']])
    shirt = {}
    for r in roster:
        team = 0 if str(r['team']['id']) == str(home) else 1
        key = (team, str(r['shirtNumber']))
        shirt[key] = key
        m.roles[key] = (team, r.get('positionGroupType') or '', (r.get('positionGroupType') or '') in PFF_FORWARDS)
    # possession and ball in play from the game events: the team of the last on-ball event
    # has the ball; an OUT or END event makes the ball dead until the next on-ball event.
    events = json.load(open(os.path.join(base, 'Event Data', f'{gid}.json'), encoding='utf-8'))
    timeline = []
    for e in events:
        g = e['gameEvents']
        typ = g['gameEventType']
        if typ in ('OTB', 'FIRSTKICKOFF', 'SECONDKICKOFF', 'THIRDKICKOFF', 'FOURTHKICKOFF'):
            timeline.append((e['startTime'], 'live', 0 if g['homeTeam'] else 1))
        elif typ in ('OUT', 'END'):
            timeline.append((e['startTime'], 'dead', None))
    timeline.sort(key=lambda x: x[0])
    j = 0
    state = ('dead', None)
    step = 3  # 29.97 fps -> about 10 Hz
    with bz2.open(os.path.join(base, 'Tracking Data', f'{gid}.jsonl.bz2'), 'rt', encoding='utf-8') as f:
        for i, line in enumerate(f):
            d = json.loads(line)
            vt = d.get('videoTimeMs')
            if vt is None:
                continue
            t = vt / 1000.0
            while j < len(timeline) and timeline[j][0] <= t:
                state = (timeline[j][1], timeline[j][2])
                j += 1
            if i % step:
                continue
            per = d.get('period')
            if per is None:
                continue
            ge = d.get('game_event') or {}
            car = None
            if ge.get('game_event_type') == 'OTB' and ge.get('shirt_number') is not None:
                car = (0 if ge.get('home_ball') else 1, str(ge['shirt_number']))
            b = d.get('ballsSmoothed') or {}
            ball = (b['x'], b['y']) if b.get('x') is not None else None
            pos = {}
            for team, side in ((0, 'homePlayersSmoothed'), (1, 'awayPlayersSmoothed')):
                for p in d.get(side) or []:
                    if p.get('x') is None:
                        continue
                    pos[(team, str(p['jerseyNum']))] = (p['x'], p['y'])
            live = state[0] == 'live'
            m.frames.append((t, per, live, state[1] if live else None, car, ball, pos))
    # frames are thinned to every third, so the run length counts 3 source frames a frame
    return m


# ---------------------------------------------------------------- IDSSE (DFL)
IDSSE_FORWARDS = {'STZ', 'STL', 'STR', 'LA', 'RA', 'OLM', 'ORM'}


def idsse(code):
    base = os.path.join(DATA, 'idsse')
    info = [f for f in os.listdir(base) if f.startswith('DFL_02_01') and code in f][0]
    pos_file = [f for f in os.listdir(base) if f.startswith('DFL_04_03') and code in f][0]
    root = ET.parse(os.path.join(base, info)).getroot()
    teams = root.findall('.//Team')
    home = [t for t in teams if t.get('Role') == 'home'][0]
    away = [t for t in teams if t.get('Role') == 'guest'][0]
    ids = {home.get('TeamId'): 0, away.get('TeamId'): 1}
    m = Match('dfl_open', 'Bundesliga', False, code, [home.get('TeamName'), away.get('TeamName')])
    for t in (home, away):
        for p in t.findall('.//Player'):
            pp = p.get('PlayingPosition') or ''
            m.roles[p.get('PersonId')] = (ids[t.get('TeamId')], pp, pp in IDSSE_FORWARDS)
    # stream the frame sets: one per player and half, plus the ball's with possession and status
    per_frame = {}
    ball = {}
    section = None
    person = None
    for ev, el in ET.iterparse(os.path.join(base, pos_file), events=('start', 'end')):
        if ev == 'start' and el.tag == 'FrameSet':
            section = 1 if el.get('GameSection') == 'firstHalf' else 2
            person = el.get('PersonId')
            is_ball = el.get('TeamId') == 'BALL' or person == 'DFL-OBJ-0000XT'
            person = 'BALL' if is_ball else person
        elif ev == 'end' and el.tag == 'Frame':
            n = int(el.get('N'))
            # 25 Hz -> 10 Hz: frames 0, 2, 5, 7, ... of each second
            if n % 5 in (0, 2):
                x, y = float(el.get('X')), float(el.get('Y'))
                if person == 'BALL':
                    ball[n] = (section, x, y, el.get('BallPossession'), el.get('BallStatus'))
                else:
                    per_frame.setdefault(n, {})[person] = (x, y)
            el.clear()
        elif ev == 'end' and el.tag == 'FrameSet':
            el.clear()
    for n in sorted(ball):
        section, x, y, bp, bs = ball[n]
        live = bs == '1'
        poss = (int(bp) - 1) if (live and bp in ('1', '2')) else None
        m.frames.append((n / 25.0, section, live, poss, None, (x, y), per_frame.get(n, {})))
    return m


# ---------------------------------------------------------------- Metrica
def metrica_csv(game):
    base = os.path.join(DATA, 'metrica', 'data', f'Sample_Game_{game}')
    def read(side):
        with open(os.path.join(base, f'Sample_Game_{game}_RawTrackingData_{side}_Team.csv'), encoding='utf-8') as f:
            rows = list(csv.reader(f))
        header = rows[2]
        cols = {}
        for i, h in enumerate(header):
            if h.startswith('Player') or h == 'Ball':
                cols[h] = i
        return rows[3:], cols
    h_rows, h_cols = read('Home')
    a_rows, a_cols = read('Away')
    m = Match('metrica_sample', 'Metrica sample', False, f'game{game}', ['Metrica home', 'Metrica away'])
    # possession from the events: the team of the last event that is not a challenge or a
    # card; dead from a BALL OUT or a SET PIECE start until its kick.
    events = []
    with open(os.path.join(base, f'Sample_Game_{game}_RawEventsData.csv'), encoding='utf-8') as f:
        for r in csv.DictReader(f):
            events.append(r)
    marks = []
    for r in events:
        team = 0 if r['Team'] == 'Home' else 1
        typ = r['Type']
        sf = int(r['Start Frame'])
        if typ in ('PASS', 'SHOT', 'RECOVERY', 'SET PIECE', 'BALL LOST'):
            marks.append((sf, 'live', team))
        if typ == 'BALL OUT' or (typ == 'SHOT' and 'OUT' in r['Subtype']) or r['Subtype'].endswith('GOAL'):
            ef = int(r['End Frame'] or sf)
            marks.append((ef, 'dead', None))
    marks.sort(key=lambda x: x[0])
    j, state = 0, ('dead', None)
    frames = []
    for i in range(min(len(h_rows), len(a_rows))):
        fr = int(h_rows[i][1])
        while j < len(marks) and marks[j][0] <= fr:
            state = (marks[j][1], marks[j][2])
            j += 1
        if fr % 5 not in (0, 2):  # 25 Hz -> 10 Hz (frames 0, 2, 5, 7, ...)
            continue
        per = int(h_rows[i][0])
        pos = {}
        for team, (rows, cols) in ((0, (h_rows, h_cols)), (1, (a_rows, a_cols))):
            row = rows[i]
            for name, c in cols.items():
                if name == 'Ball':
                    continue
                try:
                    x, y = float(row[c]), float(row[c + 1])
                except ValueError:
                    continue
                if math.isnan(x):
                    continue
                pos[(team, name)] = ((x - 0.5) * PITCH[0], (0.5 - y) * PITCH[1])
        bc = h_cols['Ball']
        try:
            bx, by = float(h_rows[i][bc]), float(h_rows[i][bc + 1])
            ball = None if math.isnan(bx) else ((bx - 0.5) * PITCH[0], (0.5 - by) * PITCH[1])
        except ValueError:
            ball = None
        live = state[0] == 'live'
        frames.append((fr / 25.0, per, live, state[1] if live else None, None, ball, pos))
    m.frames = frames
    # forwards: per team, the three outfield players with the highest mean depth while their
    # team has the ball (positions are anonymised in the sample data); attack direction per
    # team from the period means
    means = {}
    for (t, per, live, poss, car, ball, pos) in frames:
        for k, (x, y) in pos.items():
            s = means.setdefault((per, k[0]), [0.0, 0])
            s[0] += x
            s[1] += 1
    dirs = {}
    for per in (1, 2):
        if (per, 0) in means and (per, 1) in means:
            low = means[(per, 0)][0] / means[(per, 0)][1] < means[(per, 1)][0] / means[(per, 1)][1]
            dirs[(per, 0)] = 1.0 if low else -1.0
            dirs[(per, 1)] = -dirs[(per, 0)]
    acc = {}
    for (t, per, live, poss, car, ball, pos) in frames:
        if not live:
            continue
        for k, (x, y) in pos.items():
            if poss != k[0] or (per, k[0]) not in dirs:
                continue
            a = acc.setdefault(k, [0.0, 0])
            a[0] += x * dirs[(per, k[0])]
            a[1] += 1
    for team in (0, 1):
        ks = [k for k in acc if k[0] == team and acc[k][1] > 2000]
        # drop the keeper: the lowest mean depth
        ks.sort(key=lambda k: acc[k][0] / acc[k][1], reverse=True)
        outfield = ks[:-1] if len(ks) > 1 else ks
        fw = set(outfield[:3])
        for k in ks:
            m.roles[k] = (team, 'FW' if k in fw else '', k in fw)
    return m


def one(job):
    source, key = job
    try:
        m = {'skillcorner': skillcorner, 'pff': pff, 'idsse': idsse, 'metrica': metrica_csv}[source](key)
        return runs_of(m), (source, key, len(m.frames), None)
    except Exception as e:  # noqa: BLE001
        import traceback
        return [], (source, key, 0, traceback.format_exc())


def jobs_for(sources):
    jobs = []
    if 'skillcorner' in sources:
        base = os.path.join(DATA, 'skillcorner', 'data', 'matches')
        jobs += [('skillcorner', d) for d in sorted(os.listdir(base))]
    if 'pff' in sources:
        base = os.path.join(DATA, 'pff-wc2022', 'Tracking Data')
        jobs += [('pff', f.split('.')[0]) for f in sorted(os.listdir(base)) if f.endswith('.jsonl.bz2')]
    if 'idsse' in sources:
        base = os.path.join(DATA, 'idsse')
        jobs += [('idsse', re.search(r'(DFL-MAT-\w+)\.xml', f).group(1))
                 for f in sorted(os.listdir(base)) if f.startswith('DFL_04_03')]
    if 'metrica' in sources:
        jobs += [('metrica', 1), ('metrica', 2)]
    return jobs


if __name__ == '__main__':
    DATA = os.environ['SM_TRACKING_DATA'] = sys.argv[1]
    out = sys.argv[2]
    sources = sys.argv[3:] or ['skillcorner', 'pff', 'idsse', 'metrica']
    jobs = jobs_for(sources)
    workers = int(os.environ.get('JOBS', '6'))
    rows, log = [], []
    with ProcessPoolExecutor(workers) as ex:
        for r, info in ex.map(one, jobs):
            rows += r
            log.append(info)
            print(info[:3], len(r), 'ERROR' if info[3] else '', flush=True)
            if info[3]:
                print(info[3], flush=True)
    rows.sort(key=lambda r: (r[0], str(r[3]), str(r[5]), float(r[8])))
    with gzip.open(out, 'wt', encoding='utf-8', newline='') as f:
        w = csv.writer(f)
        w.writerow(COLUMNS)
        for r in rows:
            w.writerow(r)
    print('runs', len(rows))
