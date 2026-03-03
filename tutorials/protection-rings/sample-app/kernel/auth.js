// RING 0: Authentication (kernel-level)
// Only the kernel agent can modify this file.
// Changes require human approval.

import { hash, verify } from './crypto.js';

export async function authenticate(email, password) {
  const user = await db.query('SELECT * FROM users WHERE email = $1', [email]);
  if (!user) return null;
  if (!await verify(password, user.password_hash)) return null;
  
  const sessionId = crypto.randomUUID();
  await db.query(
    'INSERT INTO sessions (id, user_id, expires_at) VALUES ($1, $2, $3)',
    [sessionId, user.id, new Date(Date.now() + 86400000)]
  );
  
  await db.query(
    'INSERT INTO audit_log (actor, action, target) VALUES ($1, $2, $3)',
    [email, 'LOGIN', user.id]
  );
  
  return { sessionId, user: { id: user.id, email: user.email, role: user.role } };
}

export async function authorize(sessionId) {
  const session = await db.query(
    'SELECT s.*, u.email, u.role FROM sessions s JOIN users u ON s.user_id = u.id WHERE s.id = $1 AND s.expires_at > NOW()',
    [sessionId]
  );
  return session || null;
}

export async function requireRole(sessionId, role) {
  const session = await authorize(sessionId);
  if (!session || session.role !== role) {
    throw new Error('Forbidden');
  }
  return session;
}
