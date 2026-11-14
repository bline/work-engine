// Actual native Node ABI; no SDK mocks and no candidate/credential input.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const require = createRequire(import.meta.url);
const guard = require(process.argv[2]);
assert.equal(guard.assertProtected(), true);
await new Promise(resolve => setTimeout(resolve, 5));
assert.equal(guard.assertProtected(), true);
console.log('actual Node N-API dump protection verified');
