<!-- The player panel, ported whole from the sketch's players-profile screen: the shell with
     the player's name in the header; the sub-navigation (back to the squad, then Overview and
     the sketch's other profile views as stubs); the navy strip (age, height and build, and the
     club word built; contract, value, reputation and the staff's ability view LATER); then the
     profile grid: the positions pitch, the feet, and Body and age; the attributes by group as
     whole numbers 1 to 20 in their bands, each row listing the actions it drives on hover or
     focus; the hidden values in words with how sure the club is; personality as a summary word
     with the assistant manager's view and the mood today, never a number; effective ability as
     "Plays between N and M" with its band and his state today, then the season and the match
     ratings in `.rt` chips with one decimal. Below: his traits of play, then role fit and
     reputation and value as LATER stubs. -->
<script>
  import AppShell from '../components/AppShell.svelte';
  import Crest from '../components/Crest.svelte';
  import InfoStrip from '../components/InfoStrip.svelte';
  import SectionLabel from '../components/SectionLabel.svelte';
  import StubSection from '../components/StubSection.svelte';
  import AttributeHover from '../components/AttributeHover.svelte';
  import PositionsPitch from '../components/PositionsPitch.svelte';
  import SeasonBlock from '../components/SeasonBlock.svelte';
  import { abilityAxis, panelModel } from '../lib/player-panel.js';

  let { session } = $props();

  const TABS = [
    { id: 'back', label: 'Squad' },
    { id: 'overview', label: 'Overview', active: true },
    { id: 'development', label: 'Development', stub: true },
    { id: 'contract', label: 'Contract', menu: true, stub: true },
    { id: 'understanding', label: 'Understanding', menu: true, stub: true },
    { id: 'reports', label: 'Reports', menu: true, stub: true },
    { id: 'interaction', label: 'Interaction', menu: true, stub: true },
    { id: 'history', label: 'History', menu: true, stub: true },
  ];

  let names = $derived(session.teams ? session.teams.map((t) => t['team.name']) : ['Home', 'Away']);
  let player = $derived(session.squad[session.panelPlayer] ?? null);
  /// His part of the launcher's season record: null when the answer carries no season, and an
  /// empty season when it does not list him.
  let season = $derived.by(() => {
    const record = session.squadSeason ?? null;
    return record ? (record.players?.[player?.['player.id']] ?? {}) : null;
  });
  let model = $derived(
    panelModel(player, session.squadRatings.get(player?.['player.id']) ?? [], {
      season,
      played: session.slotPositionOf?.(session.panelPlayer) ?? null,
    })
  );

  /// The attribute whose hover is open, or null.
  let hovered = $state(null);
  const hoverId = (name) => `attribute-hover-${name}`;
  /// Opens the hover of attribute `name`; a touch does not open it.
  function openHover(name, event) {
    if (event?.pointerType === 'touch') {
      return;
    }
    hovered = name;
  }
  function closeHover(name) {
    if (hovered === name) {
      hovered = null;
    }
  }
  /// Escape closes the hover; the row keeps focus.
  function hoverKey(name, event) {
    if (event.key === 'Escape' && hovered === name) {
      event.stopPropagation();
      hovered = null;
    }
  }
  /// The class of a mood or form mark: only the mark takes a colour, never the word.
  const markClass = (token) => (token === '--good' ? 'up' : token === '--warn' ? 'down' : 'flat');

  let facts = $derived(
    model
      ? [
          { value: model.age, label: [...model.facts, model.sub].join(' · ') },
          { value: 'Contract', label: 'Wage and release clause', stub: true },
          { value: 'Value', label: 'Valuers’ range', stub: true },
          { value: 'Reputation', label: 'Across the league', stub: true },
          { value: 'Ability', label: 'The staff’s view', stub: true },
          { value: model.nation.value, label: model.nation.label || 'nationality not known' },
        ]
      : []
  );

  /// The scale of the effective-ability graphic: 12 to 20, or lower for a player below 12.
  let axis = $derived(abilityAxis(model?.plays?.lo, model?.level));
  /// Where a whole number sits on the graphic's scale, in percent.
  const at = (n) => `${Math.max(0, Math.min(100, ((n - axis.from) / (20 - axis.from)) * 100))}%`;

  function tab(id) {
    if (id === 'back') {
      session.closePlayer();
    }
  }
</script>

<AppShell
  section="squad"
  title={model?.name ?? 'Player'}
  subtitle={model ? `${model.positionWords} · ${names[0]} first team · No. ${model.shirt}` : ''}
  date={session.dateWord}
  dateSub={session.dateClock}
  action={session.action}
  onaction={() => session.act()}
  busy={session.actionBusy}
  tabs={TABS}
  ontab={tab}
  navLabel="Player views"
  menu={session.onMenu}
  menuOpen={session.menuOpen}
>
  {#snippet crest()}
    <Crest team={session.teams?.[0] ?? null} />
  {/snippet}

  {#if model}
    <div class="screen" data-screen="player" data-player={model.id}>
      <InfoStrip {facts}>
        {#snippet lead()}
          <Crest team={session.teams?.[0] ?? null} width={22} height={24} />
        {/snippet}
      </InfoStrip>

      <div class="grid">
        <div class="col first">
          <SectionLabel label="Competence by position" />
          <div data-section="positions">
            {#if model.positions}
              <PositionsPitch chips={model.positions.chips} />
              {#if model.positions.note}<p class="note" data-positions-note>{model.positions.note}</p>{/if}
            {:else}
              <p class="g">Ratings by position are not in this match’s data.</p>
            {/if}
          </div>
          <SectionLabel label="Feet" rule />
          <div data-section="feet">
            {#if model.feet}
              {#each model.feet.rows as row (row.foot)}
                <div class="foot" data-foot={row.strong ? 'strong' : 'weaker'}>
                  <p class="kv"><span>{row.text}</span><b class="num">{row.value}</b></p>
                  <div class="bar" aria-hidden="true"><i class={row.band} style:width="{row.width}%"></i></div>
                </div>
              {/each}
              <p class="note">{model.feet.note}</p>
            {:else}
              <p class="g">His feet are not in this match’s data.</p>
            {/if}
          </div>
          <SectionLabel label="Body and age" rule />
          <p class="body" data-body>
            {#if model.body.build}<span>{model.body.build}</span>{/if}
            {#if model.body.height}<span>{model.body.height}.</span>{/if}
            {#if model.body.age}<span>{model.body.age}.</span>{/if}
            {#if !model.body.height && !model.body.age}<span class="g">Height and age are not in this team file.</span>{/if}
          </p>
        </div>

        {#each model.groups as group, g (group.id)}
          <div class="col">
            <SectionLabel label={group.label} />
            <dl class="attrs">
              {#each group.rows as row (row.name)}
                <!-- The row is a tooltip trigger: it takes focus so the keyboard reaches the list
                     of actions, as the pointer does; it does nothing when pressed.
                     sdlc-debt: the a11y lint knows no focusable tooltip trigger that is not a
                     control; drop the ignore if the row becomes a disclosure button. -->
                <!-- svelte-ignore a11y_no_static_element_interactions, a11y_no_noninteractive_tabindex -->
                <div
                  class="attr"
                  data-attr={row.name}
                  tabindex="0"
                  aria-describedby={hovered === row.name ? hoverId(row.name) : undefined}
                  onpointerenter={(e) => openHover(row.name, e)}
                  onpointerleave={() => closeHover(row.name)}
                  onfocus={() => openHover(row.name)}
                  onblur={() => closeHover(row.name)}
                  onkeydown={(e) => hoverKey(row.name, e)}
                >
                  <dt>{row.label}<span class="sr"> ({row.word})</span></dt>
                  <dd class="num {row.band}">{row.value}</dd>
                  {#if hovered === row.name}
                    <AttributeHover id={hoverId(row.name)} name={row.name} label={row.label} />
                  {/if}
                </div>
              {/each}
            </dl>
            {#if g === model.groups.length - 1}
              {@render hiddenValues()}
            {/if}
          </div>
        {/each}
        <!-- An older replay carries the hidden values but no attributes: they get a column of
             their own. -->
        {#if model.groups.length === 0}
          <div class="col">
            {@render hiddenValues()}
          </div>
        {/if}

        {#snippet hiddenValues()}
          <SectionLabel label="Hidden values" />
          {#each model.hidden as value (value.name)}
            <p class="hidden" data-hidden={value.name} data-confidence={value.confidence}>
              <b>{value.label}:</b>
              {#if value.known}
                <span class="known">“{value.text}” — {value.line}</span>
              {:else}
                <span class="unknown">{value.full}</span>
              {/if}
            </p>
          {/each}
          <p class="note">Hidden: shown in words only. Words sharpen with matches at the club.</p>
        {/snippet}

        <div class="col">
          <SectionLabel label="Personality" note="summary and the assistant view" />
          <div data-section="personality" data-known={model.personality.known ? 'true' : 'false'}>
            <p class="summary" class:unknown={!model.personality.known} data-summary>{model.personality.summary}</p>
            <p class="line" class:unknown={!model.personality.known} data-sureness>{model.personality.line}</p>
            <div class="assistant" data-assistant>
              <b>Assistant manager:</b> “{model.assistant.join(' ')}”
            </div>
            <p class="mood" data-mood={model.mood.key ?? 'unknown'}>
              Mood today:
              {#if model.mood.mark}<i class="mark {markClass(model.mood.token)}" aria-hidden="true">{model.mood.mark}</i>{/if}<span
                class:unknown={!model.mood.known}>{model.mood.text}</span
              >
            </p>
            <p class="note">Hidden: eleven values act in matches. Words sharpen with matches at the club.</p>
          </div>
        </div>

        <div class="col last">
          <SectionLabel label="Effective ability" note="today" />
          {#if model.plays}
            <p class="plays" data-plays={`${model.plays.lo}-${model.plays.hi}`}>
              {#if model.plays.lo === model.plays.hi}
                Plays at <span class="cy num">{model.plays.hi}</span>
              {:else}
                Plays between <span class="cy num">{model.plays.lo}</span> and <span class="cy num">{model.plays.hi}</span>
              {/if}
            </p>
            <div class="range" aria-hidden="true">
              <i style:left={at(model.plays.lo)} style:width="calc({at(model.plays.hi)} - {at(model.plays.lo)} + 4px)"></i>
              {#if model.level !== null}<s style:left={at(model.level)}></s>{/if}
            </div>
            <div class="scale num" aria-hidden="true">{#each axis.ticks as tick (tick)}<span>{tick}</span>{/each}</div>
            <p class="note">The white line is his level when fresh. Tiredness, a lack of sharpness or a new country lower it; a hidden value never does.</p>
          {:else}
            <p class="g">Not known for this team file.</p>
          {/if}
          <div class="state" data-section="state">
            {#each model.state as row (row.key)}
              <p class="kv" data-state={row.key} data-out={row.out ? 'true' : null}>
                <span>{row.label}</span>
                {#if row.key === 'mind'}
                  <b data-mind>
                    {#if row.mood.mark}<i class="mark {markClass(row.mood.token)}" aria-hidden="true">{row.mood.mark}</i>{/if}<span
                      class:unknown={!row.mood.known}>{row.mood.text}</span
                    >
                    —
                    {#if row.form.mark}<i class="mark {markClass(row.form.token)}" aria-hidden="true">{row.form.mark}</i>{/if}<span
                      class:unknown={!row.form.known}>{row.form.text}</span
                    >
                  </b>
                {:else}
                  <b class:unknown={row.muted} class:out={row.out}>{row.text}</b>
                {/if}
              </p>
            {/each}
          </div>
          <SectionLabel label="Season" note="with us · last 6" rule />
          <SeasonBlock season={model.season} />
          <SectionLabel label="Match rating" rule />
          <div data-ratings={model.ratings.none ? 'none' : 'some'}>
            {#if model.ratings.none}
              <p class="g">{model.ratings.text}</p>
            {:else}
              <div class="chips">
                {#each model.ratings.chips as chip, c (c)}<span class="rt {chip.band}">{chip.text}</span>{/each}
              </div>
              <p class="kv"><span>Last match</span><b class="num">{model.ratings.last}</b></p>
              <p class="kv"><span>Average · {model.ratings.avg3.of}</span><b class="num">{model.ratings.avg3.text}</b></p>
              <p class="kv"><span>Average · {model.ratings.avg10.of}</span><b class="num">{model.ratings.avg10.text}</b></p>
            {/if}
          </div>
        </div>
      </div>

      <div class="lower">
        <div data-section="traits">
          <SectionLabel label="Traits of play" note="strength" />
          {#if model.traits === null}
            <p class="g">Traits of play are not in this match’s data.</p>
          {:else if model.traits.length === 0}
            <p class="g" data-traits="none">No traits of play.</p>
          {:else}
            {#each model.traits as t (t.name)}
              <p class="kv" data-trait={t.name}>
                <span>{t.words}</span><b class:strong={t.strong}>{t.strength}</b>
              </p>
            {/each}
          {/if}
          {#if model.override}
            <p class="override" data-override={model.override.count}>
              <i aria-hidden="true">!</i><span>{model.override.text}</span>
            </p>
          {/if}
        </div>
        <!-- STUB: role fit, and reputation and value come with later pieces. -->
        <StubSection note="role fit">
          <SectionLabel label="Role fit" later />
          <p class="note">His best roles of the forty, with a duty each.</p>
        </StubSection>
        <StubSection note="reputation and value">
          <SectionLabel label="Reputation and value" later />
          <p class="note">Built from his ratings, weighted by level.</p>
        </StubSection>
      </div>
    </div>
  {:else}
    <div class="screen" data-screen="player">
      <p class="g">No player is picked.</p>
    </div>
  {/if}
</AppShell>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    overflow: auto;
  }

  .grid {
    display: grid;
    grid-template-columns: 220fr 155fr 155fr 145fr 225fr 230fr;
    margin-top: 12px;
    flex: none;
  }

  .col {
    min-width: 0;
    padding: 0 14px;
  }

  .col + .col {
    border-left: 1px solid var(--rule);
  }

  .col.first {
    padding-left: 0;
  }

  .col.last {
    padding-right: 0;
  }

  .foot .kv {
    padding-bottom: 2px;
  }

  .bar {
    height: 4px;
    margin-bottom: 6px;
    border-radius: 1px;
    background: var(--bar-track);
  }

  .bar i {
    display: block;
    height: 100%;
    border-radius: 1px;
  }

  .bar i.v4 {
    background: var(--good);
  }

  .bar i.v3 {
    background: var(--mid);
  }

  .bar i.v2 {
    background: var(--warn);
  }

  .bar i.v1 {
    background: var(--bad);
  }

  .attr {
    position: relative;
    margin: 0 -4px;
    padding: 0 4px;
    border-radius: var(--radius-sm);
    cursor: default;
  }

  @media (hover: hover) and (pointer: fine) {
    .attr:hover {
      background: var(--ground-2);
    }
  }

  .summary {
    margin: 2px 0 4px;
    font: 700 15px var(--fd);
    color: var(--ink);
  }

  .line {
    margin: 0 0 10px;
    font-size: 9.5px;
    line-height: 1.4;
    color: var(--ink-2);
  }

  .assistant {
    margin: 0 0 10px;
    padding: 8px 10px;
    font-size: 10px;
    line-height: 1.45;
    color: var(--ink-2);
    background: var(--toggle-on-ground);
    border: 1px solid var(--cyan);
    border-radius: var(--radius-sm);
  }

  .assistant b {
    color: var(--ink);
    font-weight: 600;
  }

  .mood {
    margin: 0 0 6px;
    font-size: 10px;
    color: var(--ink-2);
  }

  .mark {
    font-style: normal;
    margin: 0 3px 0 2px;
  }

  .mark.up {
    color: var(--good);
  }

  .mark.down {
    color: var(--warn);
  }

  .mark.flat {
    color: var(--ink-2);
  }

  .state {
    margin-top: 10px;
  }

  .state .kv {
    border-top: 1px solid var(--rule-2);
  }

  .state .kv > span {
    flex: none;
    white-space: nowrap;
  }

  .state .kv b {
    text-align: right;
  }

  .kv b.strong {
    color: var(--cyan);
  }

  .override {
    display: flex;
    gap: 6px;
    margin: 8px 0 0;
    font-size: 9.5px;
    line-height: 1.4;
    color: var(--ink-2);
  }

  .override i {
    flex: none;
    width: 12px;
    height: 12px;
    font: 700 9px/12px var(--fd);
    font-style: normal;
    text-align: center;
    color: var(--on-warn);
    background: var(--warn);
    border-radius: var(--radius-sm);
  }

  .body {
    margin: 0;
    font-size: 9.5px;
    line-height: 1.45;
    color: var(--ink-2);
  }

  .body span + span::before {
    content: ' ';
  }

  .attrs {
    margin: 0 0 10px;
  }

  /* Each row takes focus for its hover, so it is a control: at least the hit size. */
  .attr {
    display: flex;
    justify-content: space-between;
    align-items: center;
    min-height: var(--hit);
    font-size: 10.5px;
  }

  .attr dt {
    color: var(--ink-2);
  }

  .attr dd {
    margin: 0;
    font: 700 10.5px var(--fd);
    font-variant-numeric: tabular-nums;
  }

  .v4 {
    color: var(--good);
  }

  .v3 {
    color: var(--mid);
  }

  .v2 {
    color: var(--warn);
  }

  .v1 {
    color: var(--bad);
  }

  .hidden {
    margin: 0 0 6px;
    font-size: 9.5px;
    line-height: 1.4;
  }

  .hidden b {
    color: var(--ink);
    font-weight: 600;
  }

  .known {
    color: var(--ink-2);
  }

  .unknown {
    color: var(--ink-3);
  }

  .note {
    margin: 4px 0 0;
    font-size: 9px;
    line-height: 1.4;
    color: var(--ink-3);
  }

  .plays {
    margin: 0 0 8px;
    font: 600 12px var(--fb);
  }

  .cy {
    color: var(--cyan);
  }

  .num {
    font-variant-numeric: tabular-nums;
  }

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  .range {
    position: relative;
    height: 8px;
    border-radius: 1px;
    background: var(--bar-track);
  }

  .range i {
    position: absolute;
    top: 0;
    bottom: 0;
    background: var(--cyan);
    border-radius: 1px;
  }

  .range s {
    position: absolute;
    top: -3px;
    bottom: -3px;
    width: 2px;
    background: var(--ink);
  }

  .scale {
    display: flex;
    justify-content: space-between;
    margin-top: 3px;
    font-size: 8.5px;
    color: var(--ink-3);
  }

  .chips {
    display: flex;
    gap: 4px;
    margin-bottom: 6px;
  }

  .rt {
    display: inline-block;
    min-width: 30px;
    text-align: center;
    font: 700 10px/15px var(--fd);
    border-radius: 2px;
    color: var(--on-good);
  }

  .rt.a {
    background: var(--good);
  }

  .rt.b {
    background: var(--mid);
    color: var(--on-mid);
  }

  .rt.c {
    background: var(--warn);
    color: var(--on-warn);
  }

  .rt.d {
    background: var(--bad);
    color: var(--on-bad);
  }

  .kv {
    display: flex;
    justify-content: space-between;
    margin: 0;
    padding: 3px 0;
    font-size: 10px;
    color: var(--ink-2);
  }

  .kv b {
    color: var(--ink);
    font-weight: 600;
  }

  .g {
    color: var(--ink-3);
    font-size: 10px;
  }

  .lower {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0 28px;
    margin-top: 14px;
    padding-top: 12px;
    border-top: 1px solid var(--rule);
  }

  @media (max-width: 1023px), (max-height: 599px) {
    .grid {
      grid-template-columns: repeat(3, minmax(0, 1fr));
      row-gap: 14px;
    }

    .col:nth-child(4) {
      border-left: 0;
      padding-left: 0;
    }

    .lower {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
