#!/usr/bin/env node
import { readFile } from "node:fs/promises";
import { validateExternalBootstrapPacket } from "../src/services/slice-campaign/external-bootstrap-adoption-contract.mjs";

const source = process.argv[2];
if (!source) throw new Error("usage: verify-external-bootstrap-packet.mjs <packet.json>");
const packet = validateExternalBootstrapPacket(JSON.parse(await readFile(source, "utf8")));
process.stdout.write(JSON.stringify({valid: true, whole_packet_sha256: packet.whole_packet_sha256}) + "\n");
