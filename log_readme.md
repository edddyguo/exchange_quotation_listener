15         第二次交易量2倍后下单
16        、将buy距sell的时间扩大到了2小时
17		判断了前置340根的不允许有超过当前的高度
18		在17的基础上，第二次交易量1.5倍就可以下单,前340根以内一倍改为，前159根内不能超过1.5倍
19		340根之前不能超过+第二次1.5倍
20            strategy3: 2连阴
21            strategy3: 2连阴且第二根的交易量至少是顶部的三分之一

strategy2-0001   buy时间延长，前置交易量严格
strategy3-0001   buy时间延长，前置交易量严格

strategy2-0002    剔除前置交易量限制
strategy3-0002    剔除前置交易量限制

strategy2-0003     下单前必须爬坡超过1.2
strategy3-0003      下单前必须爬坡超过1.2

## 目前结论1.2倍的拉升对胜率有小幅度提升，但是综合收益会下降