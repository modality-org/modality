import { describe, test, expect } from '@jest/globals';
import {
  SDKError,
  ConnectionError,
  TimeoutError,
  ProtocolError,
  NodeError,
} from './errors.js';

describe('Error classes', () => {
  describe('SDKError', () => {
    test('should create error with message', () => {
      const error = new SDKError('test error');
      expect(error.message).toBe('test error');
      expect(error.name).toBe('SDKError');
      expect(error).toBeInstanceOf(Error);
      expect(error).toBeInstanceOf(SDKError);
    });

    test('should have stack trace', () => {
      const error = new SDKError('test error');
      expect(error.stack).toBeTruthy();
    });
  });

  describe('ConnectionError', () => {
    test('should create error with message and peer', () => {
      const peer = '/ip4/127.0.0.1/tcp/10001';
      const error = new ConnectionError('connection failed', peer);
      expect(error.message).toBe('connection failed');
      expect(error.peer).toBe(peer);
      expect(error.name).toBe('ConnectionError');
      expect(error).toBeInstanceOf(SDKError);
    });
  });

  describe('TimeoutError', () => {
    test('should create error with message and timeout', () => {
      const error = new TimeoutError('request timeout', 5000);
      expect(error.message).toBe('request timeout');
      expect(error.timeout).toBe(5000);
      expect(error.name).toBe('TimeoutError');
      expect(error).toBeInstanceOf(SDKError);
    });
  });

  describe('ProtocolError', () => {
    test('should create error with message and response', () => {
      const response = { invalid: true };
      const error = new ProtocolError('invalid response', response);
      expect(error.message).toBe('invalid response');
      expect(error.response).toEqual(response);
      expect(error.name).toBe('ProtocolError');
      expect(error).toBeInstanceOf(SDKError);
    });
  });

  describe('NodeError', () => {
    test('should create error with message and errors object', () => {
      const errors = { code: 'TEST_ERROR', details: 'test' };
      const error = new NodeError('node error', errors);
      expect(error.message).toBe('node error');
      expect(error.errors).toEqual(errors);
      expect(error.name).toBe('NodeError');
      expect(error).toBeInstanceOf(SDKError);
    });
  });
});

