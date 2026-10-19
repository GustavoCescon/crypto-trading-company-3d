export type AgentState = 'IDLE'|'ANALYZING'|'MEETING'|'APPROVING'|'EXECUTING'|'MONITORING'|'BLOCKED'|'ERROR'
export interface Agent {id:string;name:string;role:string;room:string;state:AgentState;online:boolean;updated_at:string;heartbeat:string;tasks:number;detail:Record<string,unknown>}
export interface Ranking {symbol:string;score:number;price:string|null;volume:number;change:number;volatility:number;spread_bps:number;liquidity:number;quote_time:string;updated_at:string;trends?:Record<string,number>}
export interface Signal {strategy:string;score:number;value:number;reason:string}
export interface Decision {id:string;symbol:string;action:string;score:number;created_at:string;rules_version:string;detail:{analysis:{signals:Signal[];reason:string}}}
export interface Position {symbol:string;quantity:string;cost:string;average_price:string;price:string;value:string;unrealized_pnl:string;strategy:string;stale:boolean}
export interface Fill {id:string;intent_id:string;symbol:string;side:string;quantity:string;price:string;fee:string;realized_pnl:string;slippage_cost:string;strategy:string;created_at:string;simulated:boolean}
export interface Account {initial_balance:string;cash:string;available?:string;reserved?:string;currency?:'BRL'|'USDT';fx_rate?:string|null;fx_stale?:boolean;fx_observed_at?:string|null;equity:string;exposure:string;realized_pnl:string;unrealized_pnl:string;fees:string;drawdown:string;limits:Record<string,string|number>;simulated:boolean}
export interface CompanySettings {base_currency:'BRL';initial_capital:number;max_active_agents:number;max_active_teams:number;hires_per_management_cycle:number;management_review_minutes:number;max_positions:number;max_total_exposure_fraction:number;real_trading_enabled:false}
export interface AuditEvent {event_id:string;producer:string;occurred_at:string;sequence:number;payload:{type:string;kind?:string;data?:Record<string,unknown>}}
export interface Order {id:string;symbol:string;side:string;status:string;reason:string;risk_reason:string;created_at:string;risk_snapshot?:{sizing_explanation?:string}}
export interface News {id:string;title:string;url:string;source:string;sentiment:number;assets:string[];published_at:string;analysis:{method:string;impact:string}}
export interface Team {strategy:string;trades:number;closed:number;wins:number;realized_pnl:string;fees:string}
export interface Backtest {id:string;symbol:string;dataset_hash:string;report:Record<string,unknown>}
export interface Snapshot {schema_version:1;type:'snapshot';cursor:number;server_time:string;simulation:true;company?:{state:'STOPPED'|'STARTING'|'RUNNING'|'STOPPING'|'ERROR';message:string};settings?:CompanySettings;agents:Agent[];account:Account|null;ranking:Ranking[];positions:Position[];decisions:Decision[];fills:Fill[];orders:Order[];teams:Team[];news:News[];events:AuditEvent[];equity_history:{time:string;equity:string}[];backtests:Backtest[]}
export const stateLabels:Record<AgentState,string>={IDLE:'Aguardando',ANALYZING:'Analisando',MEETING:'Em reunião',APPROVING:'Aprovando',EXECUTING:'Executando',MONITORING:'Monitorando',BLOCKED:'Bloqueado',ERROR:'Falha'}
export const strategyLabels:Record<string,string>={moving_average:'Tendências',rsi:'RSI / reversão',macd:'MACD',bollinger:'Bollinger',momentum:'Momentum',breakout:'Breakout',committee:'Comitê'}
export function isSnapshot(value:unknown):value is Snapshot {const v=value as Partial<Snapshot>|null;return !!v&&v.schema_version===1&&v.type==='snapshot'&&v.simulation===true&&Array.isArray(v.agents)&&Array.isArray(v.ranking)&&Array.isArray(v.fills)&&typeof v.cursor==='number'}
export const money=(v:unknown)=>Number(v??0).toLocaleString('pt-BR',{minimumFractionDigits:2,maximumFractionDigits:2})
export const price=(v:unknown)=>Number(v??0).toLocaleString('en-US',{maximumFractionDigits:8})
export const clock=(v:string)=>new Date(v).toLocaleTimeString('pt-BR',{hour:'2-digit',minute:'2-digit',second:'2-digit'})
