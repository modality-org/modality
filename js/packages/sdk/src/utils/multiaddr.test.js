import { describe, test, expect } from '@jest/globals';
import { parseMultiaddr, extractPeerId, isValidConnectionAddr } from './multiaddr.js';

describe('multiaddr utilities', () => {
  describe('parseMultiaddr', () => {
    test('should parse valid multiaddr', () => {
      const addr = '/ip4/127.0.0.1/tcp/10001/ws';
      const ma = parseMultiaddr(addr);
      expect(ma).toBeTruthy();
      expect(ma.toString()).toBe(addr);
    });

    test('should parse multiaddr with peer ID', () => {
      const addr = '/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN';
      const ma = parseMultiaddr(addr);
      expect(ma).toBeTruthy();
    });

    test('should throw on invalid multiaddr', () => {
      // The multiaddr library is quite permissive, only truly invalid formats throw
      expect(() => parseMultiaddr('invalid')).toThrow();
      // Note: null and undefined are caught by our wrapper and will throw
    });
  });

  describe('extractPeerId', () => {
    test('should extract peer ID from multiaddr string', () => {
      const addr = '/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN';
      const peerId = extractPeerId(addr);
      expect(peerId).toBe('12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN');
    });

    test('should extract peer ID from multiaddr object', () => {
      const addr = '/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN';
      const ma = parseMultiaddr(addr);
      const peerId = extractPeerId(ma);
      expect(peerId).toBe('12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN');
    });

    test('should return null for multiaddr without peer ID', () => {
      const addr = '/ip4/127.0.0.1/tcp/10001/ws';
      const peerId = extractPeerId(addr);
      expect(peerId).toBe(null);
    });

    test('should return null for invalid input', () => {
      expect(extractPeerId('invalid')).toBe(null);
      // Note: null/undefined will also return null rather than throwing
    });
  });

  describe('isValidConnectionAddr', () => {
    test('should validate complete connection multiaddr', () => {
      const addr = '/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN';
      expect(isValidConnectionAddr(addr)).toBe(true);
    });

    test('should reject multiaddr without peer ID', () => {
      const addr = '/ip4/127.0.0.1/tcp/10001/ws';
      expect(isValidConnectionAddr(addr)).toBe(false);
    });

    test('should reject multiaddr without transport', () => {
      const addr = '/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN';
      expect(isValidConnectionAddr(addr)).toBe(false);
    });

    test('should reject invalid multiaddr', () => {
      expect(isValidConnectionAddr('invalid')).toBe(false);
      // Note: null/undefined will also return false
    });

    test('should accept UDP transport', () => {
      const addr = '/ip4/127.0.0.1/udp/10001/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN';
      expect(isValidConnectionAddr(addr)).toBe(true);
    });
  });
});

