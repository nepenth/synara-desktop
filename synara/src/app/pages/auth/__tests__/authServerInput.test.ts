import test from 'node:test';
import assert from 'node:assert/strict';
import { normalizeAuthServerInput } from '../authServerInput';

test('auth homeserver input keeps Matrix server names unchanged', () => {
  assert.equal(normalizeAuthServerInput(' matrix.example.org '), 'matrix.example.org');
  assert.equal(normalizeAuthServerInput('matrix.example.org:8448'), 'matrix.example.org:8448');
});

test('auth homeserver input derives a Matrix server name from an HTTP base URL', () => {
  assert.equal(normalizeAuthServerInput('https://matrix.example.org'), 'matrix.example.org');
  assert.equal(
    normalizeAuthServerInput('https://matrix.example.org:8448/deployment/'),
    'matrix.example.org:8448'
  );
});

test('auth homeserver input leaves malformed or credential-bearing URLs for validation', () => {
  assert.equal(normalizeAuthServerInput('https://'), 'https://');
  assert.equal(
    normalizeAuthServerInput('https://user:password@matrix.example.org'),
    'https://user:password@matrix.example.org'
  );
});

test('auth homeserver input keeps plain HTTP for a homeserver on this machine', () => {
  assert.equal(normalizeAuthServerInput('http://localhost:8008'), 'http://localhost:8008');
  assert.equal(normalizeAuthServerInput('http://127.0.0.1:8008/'), 'http://127.0.0.1:8008');
  assert.equal(normalizeAuthServerInput('http://[::1]:8008'), 'http://[::1]:8008');
  // Anywhere else the scheme is dropped, so discovery uses HTTPS.
  assert.equal(normalizeAuthServerInput('http://matrix.example.org'), 'matrix.example.org');
  assert.equal(normalizeAuthServerInput('https://localhost:8448'), 'localhost:8448');
});
