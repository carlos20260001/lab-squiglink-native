# Performance: what the native costs, and how it was measured

The numbers here were taken from the program itself, not guessed at. The
method is at the end, and the traps that cost the most time are at the top.

## The traps

**The app is event-driven, and that hides everything.** It asks for a
repaint only when something has changed, so a program sitting still draws
no frames and uses no CPU. Measuring it by watching the CPU says nothing.
To see the frames, the program has to be asked to keep drawing:
`ctx.request_repaint()` in `App::draw`.

**The window under Xvfb has no GPU.** It renders with llvmpipe, the
software rasteriser, and that is where nearly all the time goes. A mouse
move across the graph costs about **94 ms of rasterisation** and about
**0.7 ms of the program's own work**. The ratio is roughly 130 : 1, and it
is the same for the reference in a browser in the same environment.

So: **measure the app's own thread, not the process.** The program's thread
is the first one in `/proc/<pid>/task/`; the rasteriser's are the ones
named `llvmpipe-*`. The process total is the sum of both and says nothing
about the program.

**A click costs more than a move, and both cost more the first time.** The
first run compiles the shaders, which is why the first measurement after
starting the program is enormous. Measure twice and keep the second.

**`perf` is not available.** `perf_event_paranoid` forbids it, and the
error is "Failure to open any events for recording". The timings have to
come from the program.

**A release build is the one to judge.** The debug build spends about
five times as long in the same work, and the difference is nearly all in
text shaping, which makes the debug numbers point at the wrong thing.

## What the program costs

Measured in the release build, on the Models tab, with the curves of two
measurements on the graph:

| | |
|---|---|
| A frame, in total | **0.5 – 0.9 ms** |
| The header | 0.03 ms |
| The left column | 0.12 – 0.25 ms |
| The right column | 0.21 – 0.27 ms |
| The graph itself, of that | 0.06 ms |
| The list of brands and measurements | 0.12 – 0.22 ms |
| The equalizer's panel, when it is open | 0.30 – 0.45 ms |
| The band table's ten rows | 0.19 – 0.29 ms |
| Rebuilding the curves | 0.18 – 0.33 ms |
| Sitting still | **no CPU at all** |

A frame of half a millisecond leaves the whole of the 16.7 ms of a
sixty-frames-a-second budget spare. **There is no performance problem in
the program as it stands.**

## The one thing that would become a problem

The manager's rows are laid out for **every** curve that is on the graph,
whether or not the row is on the screen. Each row is a full row of
controls: a colour dot, the name, the key's fan, an editable field for
the offset, and five marks. So the cost of the manager grows with the
number of curves on the graph, and it grows whether you can see them or
not.

| Curves on the graph | A frame | The manager's rows |
|---|---|---|
| 2 | 0.5 ms | — |
| 10 | 0.8 ms | 0.18 ms |
| 30 | 1.9 ms | 0.76 ms |
| 60 | 1.9 ms | 1.1 ms |
| 200 | 3.9 ms | 3.2 ms |

Two hundred curves is more than the reference's whole database holds, and
3.9 ms is still inside the budget, so this is not a fault today. It is
the one place where the cost is not yet bounded by the screen.

The fix, when it is wanted, is to lay out only the rows that are visible:
`egui::ScrollArea::show_rows` takes the row's height and the number of
rows, and draws only the ones in the viewport. The rows are a fixed fifty
points tall, which is what that wants.

## The method

`App::draw` was given a clock, and each part of the frame was timed
around it — the header, the left column, the right column, and within
those the equalizer's panel, the band table's rows, and the graph's grid,
its band names and its curves. The numbers above are those, read from the
program's own report.

The measurements that needed more than the program's own report:

- **The CPU split** between the program and the rasteriser was read from
  `/proc/<pid>/task/<tid>/stat`, whose fourteenth and fifteenth fields are
  the user and system time of that thread.
- **The frames** were counted by the program itself.
- **The number of curves** was varied by choosing more measurements in the
  list, so that the manager's rows grew.

Everything here was taken out again afterwards: the timing code is not in
the tree, and the program is as it was.
