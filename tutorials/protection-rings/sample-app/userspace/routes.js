// RING 3: Feature routes (userspace)
// The userspace agent can freely modify this file.
// No approval needed.

import { authorize } from '../kernel/auth.js';

export function registerRoutes(app) {

  // Public: landing page
  app.get('/', (req, res) => {
    res.json({ message: 'Welcome to the app', version: '1.0.0' });
  });

  // Public: health check
  app.get('/health', (req, res) => {
    res.json({ status: 'ok', uptime: process.uptime() });
  });

  // Authenticated: user profile
  app.get('/profile', async (req, res) => {
    const session = await authorize(req.headers['x-session-id']);
    if (!session) return res.status(401).json({ error: 'Unauthorized' });
    res.json({ email: session.email, role: session.role });
  });

  // Authenticated: list items
  app.get('/items', async (req, res) => {
    const session = await authorize(req.headers['x-session-id']);
    if (!session) return res.status(401).json({ error: 'Unauthorized' });
    const items = await db.query('SELECT * FROM items WHERE user_id = $1', [session.user_id]);
    res.json(items);
  });

  // Authenticated: create item
  app.post('/items', async (req, res) => {
    const session = await authorize(req.headers['x-session-id']);
    if (!session) return res.status(401).json({ error: 'Unauthorized' });
    const { name, description } = req.body;
    const item = await db.query(
      'INSERT INTO items (user_id, name, description) VALUES ($1, $2, $3) RETURNING *',
      [session.user_id, name, description]
    );
    res.status(201).json(item);
  });
}
