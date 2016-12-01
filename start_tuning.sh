target/debug/sgxmusl-tuner --targ=/home/ubuntu/TUD-Work/scripts/build/build/bin/memcached \
--args2targ="-l 127.0.0.1 -p 12347" \
--bench=/usr/local/bin/mcperf \
--args2bench="-p 12349 --linger=0 --timeout=5 --conn-rate=1000 --call-rate=1000 --num-calls=10 --num-conns=1000 --sizes=u1,16" \
--maxSteps=10000 --numIter=5 --maxTemp=1000 --minTemp=2 --maxAtt=100 --maxAcc=100 --maxRej=500 \
--energy=throughput \
--cooling=exponential
