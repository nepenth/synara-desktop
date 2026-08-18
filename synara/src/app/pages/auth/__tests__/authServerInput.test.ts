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
