../target/debug/sgxmusl-autotuner --targ=/home/ubuntu/TUD-Work/wrapper-autoperf-tuning/scripts/build/master/bin/memcached \
--args2targ="-l 127.0.0.1 -p 12347 -vv" \
--bench=/usr/local/bin/mcperf \
--args2bench="-p 12349 --linger=0 --timeout=5 --conn-rate=1000 --call-rate=1000 --num-calls=10 --num-conns=1000 --sizes=u1,16" \
--maxSteps=34 --temp=10 --redFact=10 --maxAtt=10 --maxAcc=10 --maxRej=10
