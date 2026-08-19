#!/usr/bin/env node
import readline from "node:readline"; import fs from "node:fs/promises"; import { ChromeVisionBroker } from "../src/broker.mjs";
const configPath = process.argv[2]; if (!configPath) throw new Error("Usage: chrome-vision <config.json>");
const broker = new ChromeVisionBroker(JSON.parse(await fs.readFile(configPath, "utf8")));
const lines = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
for await (const line of lines) { if (!line.trim()) continue; try { console.log(JSON.stringify(await broker.request(JSON.parse(line)))); } catch (error) { console.log(JSON.stringify({ error: error.message })); } }
