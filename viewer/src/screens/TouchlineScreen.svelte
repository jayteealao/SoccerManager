<!-- The Touchline, ported from the sketch's Touchline board: the shell with TOUCHLINE and the
     fixture in the header and the match's next action in the cyan block; the fact strip (ball
     state and the next stoppage, score and clock, substitutes and windows used, mentality);
     then three columns. Left: the queued changes, the compact substitution picker with New
     shape, and the shouts and game-state plans as LATER stubs. Centre: the player state and
     "What each change did" as a stub. Right: the analysts' reads as a stub, the assistant's
     proposals and the other team's bench. Every figure is the one at the rendered tick. -->
<script>
  import AnalystReads from '../components/AnalystReads.svelte';
  import AppShell from '../components/AppShell.svelte';
  import Crest from '../components/Crest.svelte';
  import InfoStrip from '../components/InfoStrip.svelte';
  import OpponentBench from '../components/OpponentBench.svelte';
  import PlayerState from '../components/PlayerState.svelte';
  import Proposals from '../components/Proposals.svelte';
  import QueuedChanges from '../components/QueuedChanges.svelte';
  import SectionLabel from '../components/SectionLabel.svelte';
  import StubSection from '../components/StubSection.svelte';
  import SubPicker from '../components/SubPicker.svelte';
  import { mentalityWord } from '../lib/prematch.js';
  import { touchlineFacts } from '../lib/touchline.js';

  let { session } = $props();

  const TABS = [
    { id: 'match', label: 'Match' },
    { id: 'touchline', label: 'Touchline', active: true },
    { id: 'tactics', label: 'Tactics', menu: true },
    { id: 'squad', label: 'Squad', menu: true },
    { id: 'stats', label: 'Stats', menu: true, stub: true },
    { id: 'analysis', label: 'Analysis', menu: true, stub: true },
    { id: 'other-grounds', label: 'Other grounds', menu: true, stub: true },
    { id: 'highlights', label: 'Highlights', menu: true, stub: true },
  ];

  const SHOUTS = ['Encourage', 'Demand more', 'Focus', 'Calm down', 'Slow it down', 'Get stuck in'];

  let dugout = $derived(session.dugout);
  let version = $derived(dugout.version);
  let schema = $derived(dugout.schema);
  let names = $derived(session.teams ? session.teams.map((t) => t['team.name']) : ['Home', 'Away']);

  let facts = $derived.by(() => {
    version;
    const subs = session.hello?.substitutions ?? {};
    const rows = touchlineFacts({
      score: session.score,
      clock: session.clockText,
      play: session.play,
      subsUsed: session.play.subsUsed[0],
      windowsUsed: session.play.windowsUsed[0],
      limit: subs.limit ?? 0,
      windows: subs.windows ?? 0,
      mentality: mentalityWord(schema, dugout.tactics?.mentality),
    });
    // STUB: concussion substitutes need a rule the engine does not have.
    rows.splice(4, 0, { value: '—', label: 'Concussion substitutes open', stub: true });
    return rows;
  });

  /// New shape: the formations of the tactics file, the current one left out (it is "Keep").
  let shapes = $derived.by(() => {
    version;
    const list = schema?.formations ?? [];
    const current = dugout.tactics?.formation ?? 0;
    return {
      current: list[current]?.name ?? '',
      list: list.map((f, value) => ({ value, name: f.name })).filter((f) => f.value !== current),
    };
  });

  let proposals = $derived.by(() => {
    version;
    return dugout.proposals();
  });
</script>

<AppShell
  title="Touchline"
  subtitle="{session.title} · {session.subtitle}"
  date={session.dateWord}
  dateSub={session.dateClock}
  action={session.action}
  onaction={() => session.act()}
  busy={session.actionBusy}
  tabs={TABS}
  ontab={(id) => (id === 'squad' ? session.openSquad() : session.show(id))}
  menu={session.onMenu}
  menuOpen={session.menuOpen}
>
  {#snippet crest()}
    <Crest team={session.teams?.[0] ?? null} />
  {/snippet}

  <div class="screen" data-screen="touchline" data-phase={dugout.phase}>
    <InfoStrip {facts} />

    <div class="cols">
      <div class="col left">
        <QueuedChanges
          chips={dugout.chips}
          editing={dugout.editing}
          cancelRefused={dugout.cancelRefused}
          live={dugout.live}
          onedit={(id) => dugout.startEdit(id)}
          onstopedit={() => dugout.stopEdit()}
          oncancel={(id) => dugout.cancel(id)}
          ondismiss={(id) => dugout.dismiss(id)}
        />
        <div class="gap"></div>
        <SubPicker
          picker={dugout.picker}
          editing={dugout.editing?.kind === 'substitution'}
          compact
          {shapes}
          onqueue={(off, on, formation) => dugout.substitute(off, on, formation)}
        />

        <SectionLabel label="Shouts and instructions" note="start at once, spread over time" rule later />
        <!-- STUB: shouts need a model of how players take up a message; the engine has none. -->
        <StubSection note="shouts">
          <div class="shouts">
            {#each SHOUTS as s (s)}<span class="shout">{s}</span>{/each}
          </div>
        </StubSection>
        <div class="plans">
          <span class="pl">Game-state plans<StubSection note="LATER mark: game-state plans" inline fade={false} later /></span>
          <!-- STUB: game-state plans need a plan model the engine does not have. -->
          <StubSection note="game-state plans" inline>
            <span class="shout">Chase the game</span><span class="shout">Protect a lead</span>
          </StubSection>
        </div>
      </div>

      <div class="col mid">
        <PlayerState rows={dugout.playerState} clock={session.clockText} />
        <SectionLabel label="What each change did" note="since your earlier changes" rule later />
        <!-- STUB: the effect of each change needs a model the engine does not have. -->
        <StubSection note="what each change did">
          <p class="eff">Each change you make, with what it did to the match.</p>
        </StubSection>
      </div>

      <div class="col right">
        <AnalystReads name={names[1]} />
        <div class="hr"></div>
        <Proposals rows={proposals} live={dugout.live} onaccept={(pick) => dugout.accept(pick)} />
        <div class="hr"></div>
        <OpponentBench name={names[1]} bench={dugout.otherBench} />
      </div>
    </div>
  </div>
</AppShell>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  /* The board's 396 px, the rest and 392 px at 1280, as shares. */
  .cols {
    display: grid;
    grid-template-columns: minmax(0, 396fr) minmax(0, 407fr) minmax(0, 392fr);
    grid-template-rows: minmax(0, 1fr);
    margin-top: 10px;
    flex: 1;
    min-height: 0;
  }

  .col {
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    padding: 0 18px;
  }

  .col.left {
    padding-left: 0;
  }

  .col.mid,
  .col.right {
    border-left: 1px solid var(--rule);
  }

  .col.right {
    padding-right: 0;
  }

  .gap {
    height: 10px;
  }

  .hr {
    border-top: 1px solid var(--rule);
    margin: 12px 0;
  }

  .shouts {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }

  .shout {
    display: inline-block;
    height: 22px;
    line-height: 22px;
    padding: 0 10px;
    font: 600 10px var(--fb);
    color: var(--ink);
    box-shadow: inset 0 0 0 1px var(--control-edge);
    border-radius: var(--radius-sm);
    margin-right: 5px;
  }

  .plans {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 12px;
  }

  .pl {
    display: inline-flex;
    align-items: center;
    font-size: 10.5px;
    color: var(--ink-2);
  }

  .eff {
    margin: 0;
    font-size: 10.5px;
    color: var(--ink-2);
  }

  /* Compact: the changes and the assistant side by side, the player state across under
     them. The body scrolls. */
  @media (max-width: 1023px), (max-height: 599px) {
    .screen {
      height: auto;
    }

    .cols {
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
      grid-template-rows: auto auto;
      grid-template-areas: 'l r' 'm m';
      flex: none;
    }

    .col {
      overflow: visible;
    }

    .col.left {
      grid-area: l;
    }

    .col.mid {
      grid-area: m;
      border-left: 0;
      border-top: 1px solid var(--rule);
      margin-top: 14px;
      padding: 14px 0 0;
    }

    .col.right {
      grid-area: r;
      padding-right: 0;
    }
  }
</style>
