# Browser drive evidence — viewer-pitch

Driver: the in-app browser pane. Date: 2026-09-22. Every number below comes from
`window.__touchline` or from the `viewer-event` ring, read in the page after a real
pointer click on the named control.

## Panel refresh rate (bears on AC-a)

```
viewer.frame_budget  frames=300  fps_median=32  frame_ms_p95=31.62  dropped_frames=0  refresh_hz=32
```

The pane refreshes at 32 Hz. The page draws every frame the pane offers and drops none.
The 60 frames per second AC-a asks for is not reachable in this driver.

## AC-d — 4x clock within 2 percent

Server: `replay --fixture fixture.smfx --web web --speed 8`. Control: a click on `4x`.

```
tick 1991 at 16108.185 ms, clock 00:39
tick 5997 at 36132.120 ms, clock 01:59
wall 20.024 s   match 80.12 s   measured 4.001x   error 0.03 percent
```

## AC-e — the lag notice

Run 1, server `replay --speed 8 --sustain 3`, control: a click on `8x`.

```
notice     "Lag — Playing at 3x — the engine sustains three times real time"
live region "Speed 8 times asked for. Playing at 3 times."
viewer.lag  requested_speed=8  sustained_speed=3  measured_speed=2.98  tick=19455  notice_shown=true
viewer.tick_skipped  speed=3
```

Run 2, server `replay --speed 8`, control: a click on `8x`.

```
measured 7.987x over 10.02 s   notice ""   viewer.lag rows 0
```

## AC-f and AC-g — against the live engine

Server: `serve --seed 42 --minutes 90 --web web`. The live engine sends the whole match as
fast as the socket accepts it: 270,000 ticks in about 13 seconds, then it closes the
session and exits. The page stores the match and the notice reads "Stream ended".

```
viewer.history  ticks_stored=270000  history_bytes=25380000 (24.2 MB)  budget_bytes=314572800
gauge           "32 fps · 32 Hz · 25 MB"
page_bytes      null
page_bytes_reason "SecurityError: performance.measureUserAgentSpecificMemory is not available."
```

AC-f, by a click on the scrubber at about half the match:

```
scrub 131502   clock 43:52   exact true
drawn == stored for all 47 components
```
