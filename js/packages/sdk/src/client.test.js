import { describe, test, expect, beforeEach, afterEach } from '@jest/globals';
import { ModalClient, createClientOnlyConfig } from './client.js';
import { ConnectionError, NodeError, TimeoutError } from './utils/errors.js';

// Note: These are unit tests. Integration tests would require a running Modal Money node.
describe('ModalClient', () => {
  let client;

  beforeEach(() => {
    client = new ModalClient({ timeout: 5000 });
  });

  afterEach(async () => {
    if (client) {
      await client.close();
    }
  });

  describe('constructor', () => {
    test('should create client with default options', () => {
      const c = new ModalClient();
      expect(c).toBeInstanceOf(ModalClient);
      expect(c.options.timeout).toBe(30000);
      expect(c.options.clientOnly).toBe(true);
    });

    test('should create client with custom options', () => {
      const c = new ModalClient({ timeout: 10000 });
      expect(c.options.timeout).toBe(10000);
    });
  });

  describe('client-only mode', () => {
    test('should be client-only by default', () => {
      const c = new ModalClient();
      expect(c.isClientOnly()).toBe(true);
      expect(c.options.clientOnly).toBe(true);
    });

    test('should reject addresses in client-only mode', () => {
      expect(() => {
        new ModalClient({ 
          clientOnly: true, 
          addresses: { listen: ['/ip4/0.0.0.0/tcp/0'] }
        });
      }).toThrow('Cannot specify addresses in clientOnly mode');
    });

    test('should allow clientOnly: false', () => {
      const c = new ModalClient({ clientOnly: false });
      expect(c.options.clientOnly).toBe(false);
    });

    test.skip('should have no multiaddrs after init', async () => {
      const c = new ModalClient();
      await c._initLibp2p();
      
      const diagnostics = c.getClientModeDiagnostics();
      expect(diagnostics.hasListeners).toBe(false);
      expect(diagnostics.multiaddrs).toHaveLength(0);
      expect(diagnostics.clientOnly).toBe(true);
      
      await c.close();
    });

    test('should return diagnostics before init', () => {
      const c = new ModalClient();
      const diagnostics = c.getClientModeDiagnostics();
      
      expect(diagnostics.clientOnly).toBe(true);
      expect(diagnostics.hasListeners).toBe(false);
      expect(diagnostics.multiaddrs).toEqual([]);
      expect(diagnostics.connections).toBe(0);
    });
  });

  describe('createClientOnlyConfig', () => {
    test('should create config with defaults', () => {
      const config = createClientOnlyConfig();
      expect(config.clientOnly).toBe(true);
      expect(config.timeout).toBe(30000);
      expect(config.addresses).toBeUndefined();
      expect(config.listeners).toBeUndefined();
    });

    test('should allow custom timeout', () => {
      const config = createClientOnlyConfig({ timeout: 5000 });
      expect(config.timeout).toBe(5000);
      expect(config.clientOnly).toBe(true);
    });
  });

  describe('isConnected', () => {
    test('should return false when not connected', () => {
      expect(client.isConnected()).toBe(false);
    });
  });

  describe('getConnectedPeer', () => {
    test('should return null when not connected', () => {
      expect(client.getConnectedPeer()).toBe(null);
    });
  });

  describe('connect', () => {
    test('should reject invalid multiaddr', async () => {
      await expect(client.connect('invalid')).rejects.toThrow();
    });

    test('should reject multiaddr without peer ID', async () => {
      await expect(
        client.connect('/ip4/127.0.0.1/tcp/10001/ws')
      ).rejects.toThrow(ConnectionError);
    });

    test('should reject multiaddr without transport', async () => {
      await expect(
        client.connect('/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN')
      ).rejects.toThrow(ConnectionError);
    });

    // Integration test - requires running node
    test.skip('should connect to valid node', async () => {
      await client.connect('/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN');
      expect(client.isConnected()).toBe(true);
      expect(client.getConnectedPeer()).toBeTruthy();
    });
  });

  describe('ping', () => {
    test('should throw error when not connected', async () => {
      await expect(client.ping()).rejects.toThrow('Not connected');
    });

    // Integration test - requires running node
    test.skip('should successfully ping node', async () => {
      await client.connect('/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN');
      
      const response = await client.ping({ message: 'hello' });
      
      expect(response.ok).toBe(true);
      expect(response.data).toEqual({ message: 'hello' });
    });
  });

  describe('inspect', () => {
    test('should throw error when not connected', async () => {
      await expect(client.inspect()).rejects.toThrow('Not connected');
    });

    // Integration test - requires running node
    test.skip('should successfully inspect node', async () => {
      await client.connect('/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN');
      
      const response = await client.inspect({ level: 'basic' });
      
      expect(response.ok).toBe(true);
      expect(response.data).toHaveProperty('peer_id');
      expect(response.data).toHaveProperty('status');
    });

    test.skip('should pass inspection level', async () => {
      await client.connect('/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN');
      
      const response = await client.inspect({ level: 'detailed' });
      
      expect(response.ok).toBe(true);
    });
  });

  describe('request', () => {
    test('should throw error when not connected', async () => {
      await expect(client.request('/ping')).rejects.toThrow('Not connected');
    });

    // Integration test - requires running node
    test.skip('should make raw request', async () => {
      await client.connect('/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN');
      
      const response = await client.request('/ping', { test: true });
      
      expect(response.ok).toBe(true);
      expect(response.data).toEqual({ test: true });
    });
  });

  describe('close', () => {
    test('should close cleanly when not connected', async () => {
      await expect(client.close()).resolves.not.toThrow();
    });

    test.skip('should close connection', async () => {
      await client.connect('/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN');
      expect(client.isConnected()).toBe(true);
      
      await client.close();
      
      expect(client.isConnected()).toBe(false);
      expect(client.getConnectedPeer()).toBe(null);
    });
  });

  describe('getLibp2p', () => {
    test('should return null when not initialized', () => {
      expect(client.getLibp2p()).toBe(null);
    });

    test.skip('should return libp2p instance after connect', async () => {
      await client.connect('/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN');
      
      const libp2p = client.getLibp2p();
      expect(libp2p).toBeTruthy();
      expect(libp2p.start).toBeDefined();
    });
  });
});

