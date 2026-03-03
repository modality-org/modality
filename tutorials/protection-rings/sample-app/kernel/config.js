// RING 0: Application configuration (kernel-level)
// Only the kernel agent can modify this file.
// Changes require human approval.

export const config = {
  database: {
    host: process.env.DB_HOST || 'localhost',
    port: parseInt(process.env.DB_PORT || '5432'),
    name: process.env.DB_NAME || 'app',
    maxConnections: 20,
  },
  auth: {
    sessionTTL: 86400,        // 24 hours
    maxLoginAttempts: 5,
    lockoutDuration: 900,      // 15 minutes
    passwordMinLength: 12,
  },
  security: {
    corsOrigins: ['https://app.example.com'],
    rateLimitPerMinute: 60,
    csrfEnabled: true,
  },
  deploy: {
    environment: process.env.NODE_ENV || 'development',
    port: parseInt(process.env.PORT || '3000'),
  },
};
