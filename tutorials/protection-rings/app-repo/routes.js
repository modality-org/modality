// APP REPO: Feature routes
// The App Agent works freely here — no approval needed.
// It CANNOT see or access the kernel repo.
// It interacts with kernel services only through published APIs.

export function registerRoutes(app) {

  app.get('/', (req, res) => {
    res.json({ message: 'Welcome', version: '1.0.0' });
  });

  app.get('/health', (req, res) => {
    res.json({ status: 'ok', uptime: process.uptime() });
  });

  // Uses the auth API — but can't see its implementation
  app.get('/profile', async (req, res) => {
    const session = await fetch('http://kernel-service/authorize', {
      headers: { 'x-session-id': req.headers['x-session-id'] }
    }).then(r => r.json());
    if (!session) return res.status(401).json({ error: 'Unauthorized' });
    res.json({ email: session.email, role: session.role });
  });

  app.get('/items', async (req, res) => {
    // App agent writes feature code here
    const items = await db.query('SELECT * FROM items WHERE user_id = $1', [req.userId]);
    res.json(items);
  });

  app.post('/items', async (req, res) => {
    const { name, description } = req.body;
    const item = await db.query(
      'INSERT INTO items (user_id, name, description) VALUES ($1, $2, $3) RETURNING *',
      [req.userId, name, description]
    );
    res.status(201).json(item);
  });
}
