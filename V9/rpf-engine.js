const MAGIC = 0x52504637;
const BLOCK = 512;
const u32 = (v,o) => v.getUint32(o,true);
const u64 = (v,o) => v.getBigUint64(o,true);
const decoder = new TextDecoder('utf-8');

export function inspectRpf(buffer) {
  const bytes = new Uint8Array(buffer), view = new DataView(buffer);
  if (bytes.length < 32 || u32(view,0) !== MAGIC) throw new Error('Not a valid RPF7 archive (RPF7 magic not found).');
  const count=u32(view,4), namesLength=u32(view,8), encryption=u32(view,12);
  if (!count || count > 1_000_000) throw new Error(`Invalid RPF entry count: ${count}`);
  if (encryption===0x0fffffff9 || encryption===0x0fefffff) throw new Error('This archive uses AES/NG encryption and cannot be inspected by this browser parser.');
  const namesStart=16+count*16, namesEnd=namesStart+namesLength;
  if (namesEnd>bytes.length) throw new Error('RPF name table extends beyond the uploaded file.');
  const names=bytes.subarray(namesStart,namesEnd);
  const readName=(offset)=>{if(offset>=names.length)throw new Error('Invalid filename offset in RPF table.');let end=offset;while(end<names.length&&names[end]!==0)end++;return decoder.decode(names.subarray(offset,end));};
  const entries=[];
  for(let i=0;i<count;i++){
    const o=16+i*16, packed=u64(view,o), marker=u32(view,o+4), size=u32(view,o+8), flags=u32(view,o+12);
    if(marker===0x7fffff00){entries.push({index:i,type:'directory',name:readName(u32(view,o)),first:size,count:flags});continue;}
    const resource=(marker&0x80000000)!==0;
    const stored=Number((packed>>16n)&0xffffffn);
    const offset=Number((packed>>40n)&(resource?0x7fffffn:0xffffffn))*BLOCK;
    const nameOffset=Number(packed&0xffffn);
    entries.push({index:i,type:'file',name:readName(nameOffset),offset,storedSize:stored||size,uncompressedSize:size,compressed:!resource&&stored!==0,resource,resourceSystem:size,resourceGraphics:flags,flags});
  }
  if(entries[0]?.type!=='directory') throw new Error('RPF root entry is not a directory.');
  const seen=new Set([0]), rows=[];
  function walk(dir,path,depth){
    if(depth>128)throw new Error('Directory nesting exceeds safety limit.');
    const end=dir.first+dir.count;
    if(dir.first<=dir.index || end>entries.length)throw new Error(`Invalid child range in directory ${dir.name||'(root)'}.`);
    for(let i=dir.first;i<end;i++){
      if(seen.has(i))throw new Error(`Entry ${i} is referenced more than once.`);seen.add(i);
      const e=entries[i], full=path?`${path}/${e.name}`:e.name;
      const row={...e,path:full};rows.push(row);if(e.type==='directory')walk(e,full,depth+1);
    }
  }
  walk(entries[0],'',0);
  return {version:'RPF7',entryCount:count,visibleCount:rows.length,encryption,encryptionName:encryption===0x4e45504f?'OPEN':encryption===0x50584643?'CFXP':encryption===0?'Unencrypted':`Unknown (0x${encryption.toString(16)})`,headerSize:namesEnd,rows,entries};
}

export async function extractRpfEntry(buffer, entry) {
  if(entry.type!=='file') throw new Error('Directories cannot be extracted as files.');
  if(entry.resource) throw new Error('RSC7 resource extraction needs the resource decompression/virtual mapping stage; raw resource export is available.');
  const end=entry.offset+entry.storedSize;
  if(entry.offset<0||end>buffer.byteLength)throw new Error(`Entry ${entry.path} points outside the archive.`);
  const raw=buffer.slice(entry.offset,end);
  if(!entry.compressed)return raw;
  if(typeof DecompressionStream==='undefined')throw new Error('This browser does not support raw DEFLATE decompression.');
  try {
    const stream=new Blob([raw]).stream().pipeThrough(new DecompressionStream('deflate-raw'));
    const out=await new Response(stream).arrayBuffer();
    if(entry.uncompressedSize && out.byteLength!==entry.uncompressedSize)throw new Error(`Size mismatch: expected ${entry.uncompressedSize}, got ${out.byteLength}.`);
    return out;
  } catch(e){throw new Error(`Could not decompress ${entry.path}: ${e.message}`);}
}

export function formatBytes(n){if(!Number.isFinite(n))return '—';if(n<1024)return `${n} B`;const u=['KB','MB','GB','TB'];let i=-1;do{n/=1024;i++;}while(n>=1024&&i<u.length-1);return `${n.toFixed(2)} ${u[i]}`;}

// RPF7 writer: emits uncompressed standard files and preserves explicitly marked
// RSC7 resource headers/flags. It intentionally refuses encrypted archives.
const align512 = n => Math.ceil(n / BLOCK) * BLOCK;
export function buildRpf(entries, { encryption = 0x4e45504f } = {}) {
  if (![0, 0x4e45504f, 0x50584643].includes(encryption)) throw new Error('Writing encrypted RPF archives is not supported.');
  if (!Array.isArray(entries)) throw new TypeError('entries must be an array');
  const root = { name: '', type: 'directory', children: new Map() };
  for (const item of entries) {
    const parts = String(item.path || '').replaceAll('\\', '/').split('/').filter(Boolean);
    if (!parts.length || parts.some(p => p === '.' || p === '..' || p.includes('\0'))) throw new Error(`Invalid archive path: ${item.path}`);
    let dir = root;
    for (let i = 0; i < parts.length; i++) {
      const name = parts[i], key = name.toLowerCase(), last = i === parts.length - 1;
      let node = dir.children.get(key);
      if (last) {
        if (node) throw new Error(`Duplicate archive path: ${item.path}`);
        const data = item.data instanceof Uint8Array ? item.data : new Uint8Array(item.data || 0);
        dir.children.set(key, { name, type: 'file', data, resource: !!item.resource, system: item.system >>> 0 || 0, graphics: item.graphics >>> 0 || 0 });
      } else {
        if (!node) { node = { name, type: 'directory', children: new Map() }; dir.children.set(key, node); }
        if (node.type !== 'directory') throw new Error(`Path component is a file: ${parts.slice(0, i + 1).join('/')}`);
        dir = node;
      }
    }
  }
  const records = [{ node: root, parent: null }], queue = [0];
  while (queue.length) {
    const idx = queue.shift(), node = records[idx].node;
    if (node.type === 'file') continue;
    const children = [...node.children.values()].sort((a,b)=>a.name.localeCompare(b.name, 'en', { sensitivity: 'base' }));
    records[idx].first = records.length; records[idx].count = children.length;
    for (const child of children) { const ci = records.length; records.push({ node: child, parent: idx }); if (child.type === 'directory') queue.push(ci); }
  }
  if (records.length > 1_000_000) throw new Error('RPF entry count exceeds safety limit.');
  const names = [0], nameOffsets = new Map([['',0]]);
  for (const rec of records) { const n=rec.node.name; if (!nameOffsets.has(n)) { const off=names.length; if(off>0xffff)throw new Error('RPF filename table exceeds 64KB name-offset limit.'); nameOffsets.set(n,off); for(const b of new TextEncoder().encode(n))names.push(b); names.push(0); } }
  while(names.length%16)names.push(0);
  const headerLen=16+records.length*16+names.length, dataStart=align512(headerLen);
  let cursor=dataStart; const payloads=[];
  const table=new Uint8Array(16+records.length*16+names.length), dv=new DataView(table.buffer);
  dv.setUint32(0,MAGIC,true);dv.setUint32(4,records.length,true);dv.setUint32(8,names.length,true);dv.setUint32(12,encryption,true);
  for(let i=0;i<records.length;i++){
    const {node}=records[i], o=16+i*16, nameOff=nameOffsets.get(node.name);
    if(node.type==='directory') { dv.setUint32(o,nameOff,true);dv.setUint32(o+4,0x7fffff00,true);dv.setUint32(o+8,records[i].first,true);dv.setUint32(o+12,records[i].count,true); continue; }
    const data=node.data, block=cursor/BLOCK;
    if(!Number.isInteger(block)||block>(node.resource?0x7fffff:0xffffff))throw new Error('RPF archive exceeds the supported block-offset limit.');
    // The packed word stores name offset, stored byte count and 512-byte block offset.
    // RSC7 entries set bit 23 of the 24-bit block field and use the final two
    // record words for system/graphics resource sizes.
    if(data.length >= 0xffffff) throw new Error('This writer currently supports payloads smaller than 16 MiB per entry.');
    const y=BigInt(nameOff) | (BigInt(node.resource?data.length:0) << 16n) | (BigInt(block | (node.resource?0x800000:0)) << 40n);
    dv.setBigUint64(o,y,true);dv.setUint32(o+8,node.resource?node.system:data.length,true);dv.setUint32(o+12,node.resource?node.graphics:0,true);
    payloads.push({offset:cursor,data}); cursor=align512(cursor+data.length);
  }
  table.set(names,16+records.length*16);
  const out=new Uint8Array(cursor);out.set(table,0);for(const p of payloads)out.set(p.data,p.offset);
  return out.buffer;
}

export async function extractAllStandardEntries(buffer, parsed = inspectRpf(buffer)) {
  const out=[];
  for (const e of parsed.rows) if(e.type==='file'&&!e.resource) out.push({path:e.path,data:new Uint8Array(await extractRpfEntry(buffer,e))});
  return out;
}

// Preserve every entry, including raw RSC7 resource payloads, during archive rebuilds.
// Resource bytes are copied verbatim; this does not decompress or interpret RSC7 pages.
export async function extractAllArchiveEntries(buffer, parsed = inspectRpf(buffer)) {
  const out = [];
  for (const e of parsed.rows) {
    if (e.type !== 'file') continue;
    const data = e.resource
      ? buffer.slice(e.offset, e.offset + e.storedSize)
      : await extractRpfEntry(buffer, e);
    out.push({
      path: e.path,
      data: new Uint8Array(data),
      resource: !!e.resource,
      system: e.resource ? e.resourceSystem >>> 0 : 0,
      graphics: e.resource ? e.resourceGraphics >>> 0 : 0,
    });
  }
  return out;
}
