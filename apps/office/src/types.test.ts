import { describe,it,expect } from 'vitest'
import { isSnapshot,money } from './types'
describe('wire contract',()=>{it('rejects unrelated or unsupported snapshots',()=>{expect(isSnapshot({schema_version:2,type:'snapshot'})).toBe(false);expect(isSnapshot(null)).toBe(false)});it('formats decimal display without changing ledger data',()=>expect(money('1234.56')).toBe('1.234,56'))})
