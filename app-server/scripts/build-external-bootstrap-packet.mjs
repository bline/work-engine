#!/usr/bin/env node
import { readFile } from "node:fs/promises";
import { buildExternalBootstrapPacket } from "../src/services/slice-campaign/external-bootstrap-adoption-contract.mjs";

const source = process.argv[2];
if (!source) throw new Error("usage: build-external-bootstrap-packet.mjs <unsigned-packet.json>");
const input = JSON.parse(await readFile(source, "utf8"));
process.stdout.write(JSON.stringify(buildExternalBootstrapPacket(input)) + "\n");
