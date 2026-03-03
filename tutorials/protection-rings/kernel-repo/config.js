// KERNEL REPO: Configuration & Secrets
// Only the Kernel Agent + Human Admin can access this repo.

export const config = {
  database: {
    host: process.env.DB_HOST || 'localhost',
    port: parseInt(process.env.DB_PORT || '5432'),
    name: process.env.DB_NAME || 'app',
  },
  auth: {
    sessionTTL: 86400,
    maxLoginAttempts: 5,
    lockoutDuration: 900,
  },
  security: {
    corsOrigins: ['https://app.example.com'],
    rateLimitPerMinute: 60,
  },
};
