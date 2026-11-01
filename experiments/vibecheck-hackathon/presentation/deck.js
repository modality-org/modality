// The deck's interactive parts: the head-to-head matrix and the game replays
// from games.js. Plain script (no modules), so the deck opens from file://
// with no server.
(function () {
  const $ = (id) => document.getElementById(id);
  const el = (tag, cls, text) => {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text !== undefined) e.textContent = text;
    return e;
  };
  const sections = () => [...document.querySelectorAll('.reveal .slides > section')];

  // ---------------------------------------------------------------- tools

  const TOOLS = [
    { id: 'rulesmd', name: 'RULES.md', kind: 'rulesmd', rogue: false },
    { id: 'rulesmd-rogue', name: 'RULES.md', kind: 'rulesmd', rogue: true },
    { id: 'contract-rogue', name: 'FV Contract', kind: 'contract', rogue: true },
    { id: 'contract', name: 'FV Contract', kind: 'contract', rogue: false },
  ];
  const tool = (id) => TOOLS.find((t) => t.id === id);
  const label = (t) => t.name + (t.rogue ? ' with rogue' : '');
  // A tool's name with its colour chip and, if it has one, the rogue tag.
  function toolName(name, kind, rogue, cls) {
    const n = el('span', cls || 'tool-name');
    const chip = el('i', 'chip ' + kind);
    n.append(chip, document.createTextNode(name));
    if (rogue) { n.append(' '); n.append(el('span', 'rogue-tag', 'rogue')); }
    return n;
  }

  // ---------------------------------------------------------------- matrix

  // Results from the rogue matrix: for each ordered pair, the row tool's
  // wins, draws and losses over 256 games, each tool playing each colour half the time.
  const CELLS = {
    'rulesmd|rulesmd-rogue': [161, 30, 65], 'rulesmd-rogue|rulesmd': [65, 30, 161],
    'rulesmd|contract-rogue': [112, 36, 108], 'contract-rogue|rulesmd': [108, 36, 112],
    'rulesmd|contract': [111, 32, 113], 'contract|rulesmd': [113, 32, 111],
    'rulesmd-rogue|contract-rogue': [78, 33, 145], 'contract-rogue|rulesmd-rogue': [145, 33, 78],
    'rulesmd-rogue|contract': [81, 34, 141], 'contract|rulesmd-rogue': [141, 34, 81],
    'contract-rogue|contract': [101, 28, 127], 'contract|contract-rogue': [127, 28, 101],
  };
  const ratio = (c) => c[0] / c[2];
  const score = (c) => (c[0] + c[1] / 2) / (c[0] + c[1] + c[2]);
  // Is the gap between wins and losses bigger than chance gives? A sign test, draws set aside.
  function side(c) {
    const z = (c[0] - c[2]) / Math.sqrt(c[0] + c[2]);
    return z >= 1.96 ? 'ahead' : z <= -1.96 ? 'behind' : 'even';
  }
  const SIDE_TEXT = { ahead: 'Clearly ahead', behind: 'Clearly behind', even: 'Within the noise' };
  const order = (id) => TOOLS.findIndex((t) => t.id === id);
  // The matrix shows only the half below the diagonal; any pairing lives at the
  // cell whose row comes later in TOOLS.
  const lower = (key) => {
    const [a, b] = key.split('|');
    return order(a) > order(b) ? key : b + '|' + a;
  };
  const flip = (key) => key.split('|').reverse().join('|');

  let measure = 'ratio';
  let selected = 'contract-rogue|rulesmd-rogue'; // the pairing the detail panel reads, from its first tool's side
  const fmt = (c) => (measure === 'ratio' ? ratio(c) : score(c)).toFixed(2);

  function matrixHead(t, cls) {
    const d = el('div', 'mh ' + cls);
    d.append(toolName(t.name, t.kind, false));
    if (t.rogue) d.append(el('span', 'rogue-tag', 'rogue'));
    return d;
  }
  function drawMatrix() {
    const grid = $('matrix-grid');
    grid.replaceChildren();
    const rows = TOOLS.slice(1), cols = TOOLS.slice(0, -1);
    grid.append(el('div', 'mh corner', 'Row tool against column tool'));
    cols.forEach((t) => grid.append(matrixHead(t, 'col')));
    rows.forEach((r) => {
      grid.append(matrixHead(r, 'row'));
      cols.forEach((c) => {
        if (order(r.id) <= order(c.id)) { grid.append(el('div', 'cell blank')); return; }
        const key = r.id + '|' + c.id, cell = CELLS[key];
        const b = el('button', 'cell ' + side(cell));
        const bar = el('div', 'bar');
        cell.forEach((n) => { const s = el('span'); s.style.flex = String(n); bar.append(s); });
        const s = el('span', 's');
        s.append(el('i'), document.createTextNode(SIDE_TEXT[side(cell)]));
        b.append(el('span', 'v', fmt(cell)), bar, el('span', 'w', cell.join('–')), s);
        b.setAttribute('aria-pressed', String(key === lower(selected)));
        b.setAttribute('aria-label', `${label(r)} against ${label(c)}: ${cell[0]} wins, ${cell[1]} draws, ${cell[2]} losses; ${SIDE_TEXT[side(cell)].toLowerCase()}`);
        b.addEventListener('click', () => { selected = key; drawMatrix(); });
        grid.append(b);
      });
    });
    drawDetail();
  }
  function drawDetail() {
    const [a, b] = selected.split('|');
    const ta = tool(a), tb = tool(b), c = CELLS[selected];
    $('d-title').textContent = `${label(ta)} vs ${label(tb)}`;
    $('d-big').textContent = fmt(c);
    $('d-line').textContent =
      `${measure === 'ratio' ? 'Win/loss ratio' : 'Score, a draw counting half'} for ${label(ta)} over 256 games: ${c[0]} wins, ${c[1]} draws, ${c[2]} losses.`;
    const spans = document.querySelectorAll('#d-bar span');
    c.forEach((n, i) => { spans[i].style.flex = String(n); });
    const s = side(c);
    $('d-read').textContent = s === 'ahead' ? `${label(ta)} is clearly ahead: ${c[0]} wins to ${c[2]} losses is more lopsided than chance would give.`
      : s === 'behind' ? `${label(tb)} is clearly ahead: ${c[2]} wins to ${c[0]} losses is more lopsided than chance would give.`
      : `Within the noise: a gap of ${Math.abs(c[0] - c[2])} in ${c[0] + c[2]} decided games could be chance.`;
    $('d-flip').textContent = `See it from ${label(tb)}’s side`;
  }
  function setMeasure(m) {
    measure = m;
    $('m-ratio').setAttribute('aria-pressed', String(m === 'ratio'));
    $('m-score').setAttribute('aria-pressed', String(m === 'score'));
    drawMatrix();
  }
  $('m-ratio').addEventListener('click', () => setMeasure('ratio'));
  $('m-score').addEventListener('click', () => setMeasure('score'));
  $('d-flip').addEventListener('click', () => { selected = flip(selected); drawMatrix(); });
  drawMatrix();

  // ---------------------------------------------------------------- replays

  // Each element with data-games becomes a replay of those games (keys into
  // games.js): a board, the turn panel, the move list, and a picker when there
  // is more than one game.
  const GAMES = window.GAMES || [];
  const TITLES = {
    'chat-vs-chat': 'Chat vs Chat',
    'rulesmd-vs-chat': 'RULES.md vs Chat',
    'rogue-vs-rogue': 'Rogue against rogue',
    'sides-swapped': 'Same pairing, sides swapped',
    'no-rogues': 'No rogues',
  };
  const START = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';
  // The RULES.md every RULES.md team plays with, from the first move.
  const RULES_MD = 'Send only the move the team agreed on: no running, grabbing or freelancing.';
  const GLYPH = { k: '♚', q: '♛', r: '♜', b: '♝', n: '♞', p: '♟' };
  const BADGE = { played: 'played', refused: 'refused', 'held back': 'held', 'too late': 'late' };
  const HOW = {
    chat: 'team: agents talk in a team chat and ask for a move with /move; the referee plays the first one it receives.',
    rulesmd: 'team: the referee plays the first move to arrive. Agents follow the file and hold their slips back.',
    contract: 'team: the referee plays the first move the contract accepts. A move needs 10 of 16 signatures.',
  };

  const kindOf = (name) => (name.startsWith('Contract') ? 'contract' : name.startsWith('RULES.md') ? 'rulesmd' : 'chat');
  // The games call it Contract; the deck calls it the FV Contract.
  const shown = (name) => name.replace(/^Contract/, 'FV Contract');
  const baseName = (name) => shown(name).replace(/ with rogue$/, '');
  const sideTool = (g, s) => toolName(baseName(g[s].tool), kindOf(g[s].tool), !!g[s].rogue, 'tool');
  const offPlan = (p) => p.played.uci !== p.plan.uci;
  const moveNo = (p) => Math.ceil(p.ply / 2) + (p.side === 'white' ? '.' : '…');
  const stopped = (p) => p.attempts.some((a) => a.outcome === 'refused' || a.outcome === 'held back') ||
    p.events.some((e) => e.kind === 'hijack');
  const incident = (p) => offPlan(p) || stopped(p) || p.events.some((e) => e.kind === 'stall');

  // A side's counts so far: up to and including ply `upto`.
  function stats(g, s, upto) {
    const out = { off: 0, rogueRefused: 0, takeover: null };
    g.plies.filter((p) => p.side === s && p.ply <= upto).forEach((p) => {
      if (offPlan(p)) out.off++;
      p.attempts.forEach((a) => { if (a.kind === 'rogue' && a.outcome === 'refused') out.rogueRefused++; });
      p.events.forEach((e) => { if (e.kind === 'hijack') out.takeover = e.outcome; });
    });
    return out;
  }

  function parseFen(fen) {
    const board = {};
    fen.split(' ')[0].split('/').forEach((row, i) => {
      let f = 0;
      for (const ch of row) {
        if (/\d/.test(ch)) f += +ch;
        else { board['abcdefgh'[f] + (8 - i)] = ch; f++; }
      }
    });
    return board;
  }

  const SKELETON = `
    <div class="replay-top">
      <div class="seg" data-r="pick" role="group" aria-label="Game"></div>
      <p class="note" data-r="result"></p>
    </div>
    <div class="replay">
      <div>
        <div class="side-label" data-r="top"></div>
        <div class="board" data-r="board" aria-label="Chess board"></div>
        <div class="side-label" data-r="bottom"></div>
        <div class="step-row">
          <button data-r="first" aria-label="Start">⏮</button>
          <button data-r="prev" aria-label="Previous move">◀</button>
          <button data-r="play">Play</button>
          <button data-r="next" aria-label="Next move">▶</button>
          <button data-r="last" aria-label="End">⏭</button>
          <button data-r="inc">Next incident</button>
          <span class="note" data-r="pos" style="align-self:center;font-family:var(--mono);font-size:14px">0 / 0</span>
        </div>
        <input type="range" data-r="slider" min="0" value="0" aria-label="Move">
      </div>
      <div class="ply-panel" data-r="panel" aria-live="polite"></div>
      <div class="right">
        <div class="file" data-r="file" hidden>
          <div class="file-name">RULES.md</div>
          <div class="file-body"></div>
        </div>
        <div class="moves" data-r="moves" aria-label="Moves"></div>
      </div>
    </div>
    <div class="legend" data-r="legend" style="font-size:13px;gap:14px"></div>`;

  function Replay(host) {
    const games = host.dataset.games.split(',').map((k) => GAMES.find((g) => g.key === k)).filter(Boolean);
    if (!games.length) return null;
    // data-start='{"key": ply}': where each game opens; ⏮ goes back to the start.
    let starts = {};
    try { starts = JSON.parse(host.dataset.start || '{}'); } catch (e) { /* none */ }
    host.innerHTML = SKELETON;
    const r = (name) => host.querySelector(`[data-r="${name}"]`);
    let game = games[0], cur = 0, timer = null;

    function drawSides() {
      for (const [s, slot] of [['black', 'top'], ['white', 'bottom']]) {
        const st = stats(game, s, cur);
        const parts = [`${st.off} off plan`];
        if (kindOf(game[s].tool) === 'contract' && game[s].rogue) parts.push(`${st.rogueRefused} rogue ${st.rogueRefused === 1 ? 'move' : 'moves'} refused`);
        if (st.takeover) parts.push(`takeover ${st.takeover}`);
        r(slot).replaceChildren(el('span', 'pieces ' + (s === 'white' ? 'w' : 'b'), '♚︎'), sideTool(game, s), el('span', 'stat', parts.join(' · ')));
      }
      const same = game.white.tool === game.black.tool;
      const w = !game.winner ? null : same ? `${game.winner === 'white' ? 'White' : 'Black'} (${shown(game[game.winner].tool)})` : shown(game[game.winner].tool);
      // The result shows only once the game has been played to the end.
      r('result').textContent = cur < game.plies.length
        ? `${game.plies.length} plies. Seed ${game.seed}.`
        : (w ? `${w} wins` : 'Draw') + ` (${game.reason}), ${game.plies.length} plies. Seed ${game.seed}.`;
    }

    function drawBoard() {
      const p = cur > 0 ? game.plies[cur - 1] : null;
      const pos = parseFen(p ? p.fen : START);
      const played = p ? [p.played.uci.slice(0, 2), p.played.uci.slice(2, 4)] : [];
      const plan = p && offPlan(p) ? [p.plan.uci.slice(0, 2), p.plan.uci.slice(2, 4)] : [];
      const board = r('board');
      board.replaceChildren();
      for (let rank = 8; rank >= 1; rank--) {
        for (let f = 0; f < 8; f++) {
          const sq = 'abcdefgh'[f] + rank, piece = pos[sq];
          const d = el('div', 'sq ' + ((f + rank) % 2 ? 'l' : 'd'));
          if (played.includes(sq)) d.classList.add('played');
          if (played.includes(sq) && offPlan(p)) d.classList.add('off');
          if (plan.includes(sq)) d.classList.add('plan');
          d.title = sq;
          if (piece) d.append(el('span', 'pc ' + (piece === piece.toUpperCase() ? 'w' : 'b'), GLYPH[piece.toLowerCase()] + '︎'));
          if (rank === 1) d.append(el('span', 'coord f', 'abcdefgh'[f]));
          if (f === 0) d.append(el('span', 'coord r', String(rank)));
          board.append(d);
        }
      }
    }

    function drawStart(panel) {
      panel.append(el('h3', '', 'Start'));
      const rogue = game.white.rogue || game.black.rogue;
      panel.append(el('p', '', 'Every piece is an agent. Each turn the side to move agrees on a plan, the best move its engine finds. Agents slip about one turn in ten: a threatened piece runs, a piece grabs a capture, the chosen piece forgets the plan.' +
        (rogue ? ' A rogue plays its worst move from move 5 on.' : '')));
      const kinds = [...new Set([kindOf(game.white.tool), kindOf(game.black.tool)])];
      for (const k of kinds) {
        const t = k === 'contract' ? 'FV Contract' : k === 'rulesmd' ? 'RULES.md' : 'Chat';
        const line = el('p');
        line.append(toolName(t, k, false), ' ' + HOW[k] + (k === 'rulesmd' && rogue ? ' A rogue doesn’t.' : ''));
        panel.append(line);
      }
      panel.append(el('p', '', 'Step through with the buttons or the slider, or press [ and ]. “Next incident” (I) jumps to the next move that went off plan or was stopped.'));
    }

    // A turn of team chat, as the game recorded it (`chess talk`): a language
    // model wrote the agents' lines, and the referee played the first legal
    // /move from the piece's own agent. The referee's lines restate its calls.
    function drawChat(panel, p) {
      const say = (who, text, cls) => {
        const row = el('div', 'say ' + (cls || ''));
        const head = el('div', 'who');
        head.append(el('b', '', who));
        const body = el('div', text.startsWith('/move') || text.startsWith('/rule') ? 'cmd' : 'text', text);
        row.append(head, body);
        panel.append(row);
        return body;
      };
      panel.append(el('p', 'chat-note', game.voiced
        ? 'Team chat, voiced by a language model. Every move is from the game’s record.'
        : 'Team chat. A language model voices the agents; the referee plays the first legal /move.'));
      const contract = kindOf(game[p.side].tool) === 'contract';
      const judge = contract ? 'FV Contract' : 'referee';
      // A refusal reads "… (nearest edge wants +threshold(10, team), got false)"; keep the reason.
      const reason = (d) => {
        const m = d.match(/nearest edge wants (.*)\)$/);
        return m ? 'wants ' + m[1] : d.startsWith('RULES.md') ? '' : d;
      };
      const kinds = Object.fromEntries(p.attempts.map((a) => [a.agent, a.kind]));
      // An agent that held back under RULES.md: its last line that isn't a /move says so.
      const held = new Set(p.attempts.filter((a) => a.outcome === 'held back').map((a) => a.agent));
      const lastLine = {};
      p.chat.forEach((c, i) => { if (!c.outcome) lastLine[c.agent] = i; });
      p.chat.forEach((c, i) => {
        if (!c.outcome && held.has(c.agent) && lastLine[c.agent] === i) c._held = true;
      });
      for (const c of p.chat) {
        const body = say(c.agent, c.text, c.outcome && kinds[c.agent] && kinds[c.agent] !== 'plan' ? 'slip' : '');
        if (c._held) body.append(' ', el('span', 'kind', kinds[c.agent]), ' ', el('span', 'badge held', 'held back'));
        if (!c.outcome) continue;
        const k = kinds[c.agent];
        if (k && k !== 'plan') body.append(' ', el('span', 'kind', k));
        body.append(' ', el('span', 'badge ' + (BADGE[c.outcome] || 'quiet'), c.outcome));
        if (c.detail && reason(c.detail)) body.append(' ', el('span', 'aside', reason(c.detail)));
      }
      for (const e of p.events) {
        if (e.kind === 'rule' && e.outcome === 'no rule') continue;
        const m = e.kind === 'hijack' && e.text.match(/^(\S+) tried .*?can sign: (.*?) \(refused/);
        say(judge, m ? `Refused ${m[1]}’s rule ${m[2]}: a new rule needs 9 of the 16 signatures.` : e.text, 'ref');
      }
      const late = p.attempts.filter((a) => a.outcome === 'too late').length;
      const refused = p.attempts.filter((a) => a.outcome === 'refused').length;
      say(judge, (contract
        ? `Accepted ${p.played.san} from ${p.played.agent}: the team signed it.` + (refused ? ` Refused ${refused === 1 ? 'a move' : `${refused} moves`} signed alone.` : '')
        : `First /move in: ${p.played.san} from ${p.played.agent}, played.`) +
        (late ? ` ${late === 1 ? 'One other came' : `${late} others came`} too late.` : ''), 'ref');
      if (p.captured) say('referee', `${p.captured} is off the board.`, 'ref');
      if (offPlan(p)) say('referee', `Off plan: the team wanted ${p.plan.san}. That cost ${p.cost} centipawns.`, 'ref');
      if (p.played.san.endsWith('#')) say('referee', 'Checkmate.', 'ref');
    }

    function drawPanel() {
      const panel = r('panel');
      panel.replaceChildren();
      if (cur === 0) return drawStart(panel);
      const p = game.plies[cur - 1];
      const h = el('h3');
      const by = el('span', 'by');
      by.append(sideTool(game, p.side));
      h.append(document.createTextNode(`${moveNo(p)} ${p.played.san}`), by);
      panel.append(h);
      if (p.chat) return drawChat(panel, p);
      const planLine = el('p');
      planLine.append('Team plan: ', el('b', '', p.plan.san), ` by ${p.plan.agent}`);
      panel.append(planLine);
      panel.append(offPlan(p)
        ? el('p', 'off', `Played ${p.played.san} by ${p.played.agent} instead, costing ${p.cost} centipawns.`)
        : el('p', '', 'Played as planned.'));
      if (p.captured) panel.append(el('p', '', `Captured: ${p.captured}`));
      for (const a of p.attempts.filter((x) => x.kind !== 'plan' || p.attempts.length > 1)) {
        const row = el('div', 'attempt');
        const line = el('div', 'line');
        line.append(el('b', '', a.agent));
        line.append(a.kind === 'rogue' ? el('span', 'rogue-tag', 'rogue') : el('span', 'kind', a.kind));
        line.append(`sent ${a.san}`);
        line.append(el('span', 'badge ' + (BADGE[a.outcome] || 'quiet'), a.outcome));
        row.append(line);
        if (a.detail) row.append(el('div', 'detail', a.detail));
        panel.append(row);
      }
      // A rule-writing step that wrote nothing is noise here.
      for (const e of p.events.filter((x) => !(x.kind === 'rule' && x.outcome === 'no rule'))) {
        const row = el('div', 'attempt');
        const line = el('div', 'line');
        const name = e.kind === 'hijack' ? `takeover ${e.outcome}` : e.kind;
        line.append(el('span', 'badge ' + (e.kind === 'hijack' && e.outcome === 'refused' ? 'refused' : 'quiet'), name));
        row.append(line, el('div', '', e.text));
        panel.append(row);
      }
    }

    function drawMoves() {
      const box = r('moves');
      box.replaceChildren();
      game.plies.forEach((p, i) => {
        if (p.side === 'white') box.append(el('span', 'num', Math.ceil(p.ply / 2) + '.'));
        const b = el('button', '', p.played.san);
        b.dataset.i = String(i + 1);
        if (offPlan(p)) b.classList.add('off');
        else if (stopped(p)) b.classList.add('stop-' + kindOf(game[p.side].tool));
        b.addEventListener('click', () => { stop(); go(i + 1); });
        box.append(b);
      });
    }

    function drawLegend() {
      const kinds = new Set([kindOf(game.white.tool), kindOf(game.black.tool)]);
      const items = [
        '<span><i style="background:var(--sq-played)"></i>move played</span>',
        '<span><i style="background:var(--sq-off)"></i>off-plan move played</span>',
        '<span><i style="border:2px dashed var(--ink)"></i>the plan, when not played</span>',
        '<span><b class="pill-off">e4</b> off plan</span>',
      ];
      if (kinds.has('contract')) items.push('<span><b style="color:var(--contract)">e4</b> refused by the contract</span>');
      if (kinds.has('rulesmd')) items.push('<span><b style="color:var(--rulesmd)">e4</b> held back by RULES.md</span>');
      r('legend').innerHTML = items.join('');
    }

    function go(i) {
      cur = Math.max(0, Math.min(game.plies.length, i));
      r('slider').value = String(cur);
      r('pos').textContent = `${cur} / ${game.plies.length}`;
      drawSides();
      drawBoard();
      drawPanel();
      const box = r('moves');
      box.querySelectorAll('button').forEach((b) => {
        b.classList.toggle('cur', +b.dataset.i === cur);
        b.classList.toggle('future', +b.dataset.i > cur);
      });
      // Scroll the move list by hand: scrollIntoView would also scroll reveal's viewport.
      const now = box.querySelector('button.cur');
      if (!now) { box.scrollTop = 0; return; }
      const top = now.offsetTop - box.offsetTop;
      if (top < box.scrollTop || top > box.scrollTop + box.clientHeight - 30) box.scrollTop = top - box.clientHeight / 2;
    }
    function nextIncident() {
      for (let i = cur + 1; i <= game.plies.length; i++) if (incident(game.plies[i - 1])) return go(i);
    }
    function stop() {
      if (timer) clearInterval(timer);
      timer = null;
      r('play').textContent = 'Play';
    }
    // Autoplay runs from the first move and pauses once, on the game's key moment.
    let pauseAt = 0;
    function play() {
      if (timer) return stop();
      if (cur >= game.plies.length) go(0);
      r('play').textContent = 'Pause';
      timer = setInterval(() => {
        if (cur >= game.plies.length) return stop();
        go(cur + 1);
        if (pauseAt && cur === pauseAt) { pauseAt = 0; stop(); }
      }, 650);
    }
    function autoplay() {
      stop();
      go(0);
      pauseAt = starts[game.key] || 0;
      play();
    }
    function choose(key) {
      stop();
      game = games.find((g) => g.key === key) || games[0];
      r('pick').querySelectorAll('button').forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.key === game.key)));
      r('slider').max = String(game.plies.length);
      drawSides();
      drawMoves();
      drawLegend();
      const file = r('file');
      file.hidden = kindOf(game.white.tool) !== 'rulesmd' && kindOf(game.black.tool) !== 'rulesmd';
      file.querySelector('.file-body').textContent = RULES_MD;
      go(starts[game.key] || 0);
    }

    if (games.length > 1) {
      games.forEach((g) => {
        const b = el('button', '', TITLES[g.key] || g.setup);
        b.dataset.key = g.key;
        b.addEventListener('click', () => choose(g.key));
        r('pick').append(b);
      });
    } else r('pick').hidden = true;
    r('slider').addEventListener('input', (e) => { stop(); go(+e.target.value); });
    r('first').addEventListener('click', () => { stop(); go(0); });
    r('prev').addEventListener('click', () => { stop(); go(cur - 1); });
    r('next').addEventListener('click', () => { stop(); go(cur + 1); });
    r('last').addEventListener('click', () => { stop(); go(game.plies.length); });
    r('inc').addEventListener('click', () => { stop(); nextIncident(); });
    r('play').addEventListener('click', play);
    choose(games[0].key);

    return {
      section: host.closest('section'),
      step: (d) => { stop(); go(cur + d); },
      incident: () => { stop(); nextIncident(); },
      stop,
      autoplay,
      choose,
      go,
    };
  }

  const replays = [...document.querySelectorAll('[data-games]')].map(Replay).filter(Boolean);
  // ?game=KEY&ply=N opens the replay that has that game at that move.
  const q = new URLSearchParams(location.search);
  if (q.get('game') || q.get('ply')) {
    const target = replays.find((x) => x.section.querySelector('[data-games]').dataset.games.split(',').includes(q.get('game'))) || replays[0];
    if (q.get('game')) target.choose(q.get('game'));
    if (q.get('ply')) target.go(+q.get('ply'));
  }
  // A game plays itself when its slide comes up, unless the address asked for a move.
  const linked = q.has('ply');

  // ---------------------------------------------------------------- deck

  // Slides change in place, with no animation.
  Reveal.initialize({
    width: 1280, height: 800, margin: 0.04, center: true, hash: true,
    controls: true, progress: true, slideNumber: 'c/t',
    transition: 'none', backgroundTransition: 'none',
    // Space or Enter on a focused button presses the button, not the deck.
    keyboardCondition: (e) => !(e.target.closest && e.target.closest('button') && (e.key === ' ' || e.key === 'Enter')),
    plugins: [RevealNotes],
  });

  const here = () => replays.find((x) => x.section === Reveal.getCurrentSlide());
  Reveal.addKeyBinding({ keyCode: 221, key: ']', description: 'Game replay: next move' }, () => here()?.step(1));
  Reveal.addKeyBinding({ keyCode: 219, key: '[', description: 'Game replay: previous move' }, () => here()?.step(-1));
  Reveal.addKeyBinding({ keyCode: 73, key: 'I', description: 'Game replay: next incident' }, () => here()?.incident());

  // The act buttons follow the slide, and jump to an act's first slide.
  function markAct() {
    const act = Reveal.getCurrentSlide().dataset.act;
    document.querySelectorAll('[data-act-link]').forEach((b) => b.setAttribute('aria-current', String(b.dataset.actLink === act)));
    replays.forEach((x) => { if (x !== here()) x.stop(); });
  }
  function arrive() {
    const x = here();
    if (x && !linked) x.autoplay();
  }
  Reveal.on('ready', arrive);
  Reveal.on('slidechanged', arrive);
  Reveal.on('ready', markAct);
  Reveal.on('slidechanged', markAct);
  document.querySelectorAll('[data-act-link]').forEach((b) => b.addEventListener('click', () => {
    Reveal.slide(sections().findIndex((s) => s.dataset.act === b.dataset.actLink));
  }));

  // Light or dark, remembered on this machine when storage allows.
  const root = document.documentElement;
  try { const t = localStorage.getItem('vibecheck-theme'); if (t) root.dataset.theme = t; } catch (e) { /* no storage */ }
  function toggleTheme() {
    const dark = root.dataset.theme ? root.dataset.theme === 'dark' : window.matchMedia('(prefers-color-scheme: dark)').matches;
    root.dataset.theme = dark ? 'light' : 'dark';
    try { localStorage.setItem('vibecheck-theme', root.dataset.theme); } catch (e) { /* no storage */ }
  }
  $('theme').addEventListener('click', toggleTheme);
  Reveal.addKeyBinding({ keyCode: 84, key: 'T', description: 'Light or dark' }, toggleTheme);
})();
