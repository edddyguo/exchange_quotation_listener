将信号单独封装起来，好做组合
构建一个基本买卖的策略，好单独验证其他信号的有效性。
（一根开卖，2小时之后平仓）

每个策略用枚举

多细分形态单独数据统计，放在一个进程里：
多细分策略集合：
sell
1、高点三连阴，bar形态降低，交易量要求降低
2、短时间内2次，触发卖信号，
3、高位砸盘信号明显的：形态、量都搞要求
4、出现信号后，最近10根横盘或者下探的，多交易量有要求

buy
1、短期3根内交易量大幅度萎缩的
2、中期交易量萎缩+上扬的
3、


常用cmd
```
cat forever_7_top_150_wait_6h_3month_2022_2023.log |grep  month |grep -v tmp|awk -F "finally" '{print $2}'|grep Tw
cat forever_7_top_150_wait_6h_3month_2022_2023.log|grep MAX|grep "O 3"|grep -v "O 3\."
cat 1.txt |grep '\$' -B1|grep -E '^[A-Z]+$' > 2022_05.txt;echo > 1.txt
```

