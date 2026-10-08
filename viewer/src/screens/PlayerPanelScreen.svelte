<!-- The player panel, ported whole from the sketch's players-profile screen (boards 3 and 4): the
     shell with the player's name in the header; the sub-navigation (back to the squad, then
     Overview and the sketch's other profile views as stubs); the navy strip (age, height and
     build, and the club word built; contract, value, reputation and the staff's ability view
     LATER); then the profile grid: competence by position and feet (LATER) over Body and age;
     the attributes by group as whole numbers 1 to 20 in their bands; the hidden values in
     words with how sure the club is; personality (LATER); effective ability as "Plays between
     N and M" with its band, and the match ratings in `.rt` chips with one decimal. Habits of
     play, role fit, and reputation and value stay as LATER stubs. -->
<script>
  import AppShell from '../components/AppShell.svelte';
  import Crest from '../components/Crest.svelte';
  import InfoStrip from '../components/InfoStrip.svelte';
  import SectionLabel from '../components/SectionLabel.svelte';
  import StubSection from '../components/StubSection.svelte';
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
  let model = $derived(panelModel(player, session.squadRatings.get(player?.['player.id']) ?? []));

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
          <!-- STUB: competence by position and feet need models the engine does not have. -->
          <StubSection note="competence by position">
            <SectionLabel label="Competence by position" later />
            <div class="pitch"></div>
          </StubSection>
          <StubSection note="feet">
            <SectionLabel label="Feet" later />
            <div class="feet"></div>
          </StubSection>
          <SectionLabel label="Body and age" />
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
                <div class="attr" data-attr={row.name}>
                  <dt>{row.label}<span class="sr"> ({row.word})</span></dt>
                  <dd class="num {row.band}">{row.value}</dd>
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
          <!-- STUB: personality needs a model the engine does not have yet. -->
          <StubSection note="personality">
            <SectionLabel label="Personality" later />
            <p class="note">Thirteen personality attributes, in words, with how sure the staff are.</p>
          </StubSection>
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
        <!-- STUB: habits of play, role fit, and reputation and value come with later pieces. -->
        <StubSection note="habits of play">
          <SectionLabel label="Habits of play" later />
          <p class="note">How often he does a thing against similar players.</p>
        </StubSection>
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

  .pitch {
    height: 120px;
    border-radius: var(--radius-sm);
    background: var(--pitch);
    margin-bottom: 12px;
  }

  .feet {
    height: 40px;
    margin-bottom: 12px;
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

  .attr {
    display: flex;
    justify-content: space-between;
    align-items: center;
    height: 21px;
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
