import {mkdir,copyFile,rm} from 'node:fs/promises';
await rm('dist',{recursive:true,force:true}); await mkdir('dist',{recursive:true});
for (const f of ['index.html','src.js','style.css']) await copyFile(f,`dist/${f}`);
console.log('Static build complete: dist/');
