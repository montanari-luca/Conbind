#!/usr/bin/env node
import { program } from 'commander';
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { registerValidate } from './commands/validate';
import { registerBuild } from './commands/build';
import { registerLink, registerUnlink } from './commands/link';
import { registerDev } from './commands/dev';
import { registerDoctor } from './commands/doctor';
import { registerAttach } from './commands/attach';
import { registerDetach } from './commands/detach';
import { registerPublish } from './commands/publish';

const pkg = JSON.parse(readFileSync(resolve(__dirname, '..', '..', 'package.json'), 'utf-8'));

program.name('conbind').description('Conbind Extension Developer CLI').version(pkg.version);

registerValidate(program);
registerBuild(program);
registerLink(program);
registerUnlink(program);
registerDev(program);
registerDoctor(program);
registerAttach(program);
registerDetach(program);
registerPublish(program);

program.parse(process.argv);
