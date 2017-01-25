#!/bin/bash

#==============================================================================#
set -e
# set -x

#============================== PARAMETERS ====================================#
bm="memcached"
resfile="memaslap.log"

sshhost="localhost"
maxtries="1"
remotedir="/home/dimakuv/code/sgx/sgx-guardian/bounds-checker/benches/other/memcached/build"
duration="20"
[ ! -z "$1" ] && sshhost="$1"
[ ! -z "$2" ] && maxtries="$2"

ssh="ssh ${sshhost}"
if [ "$sshhost" = 'localhost' -o "$sshhost" = "127.0.0.1" ]; then
    ssh=""
fi

declare -a clientsarr=(16 32 64 96 128 160 192 224 256 320 384 512)
declare -a typesarr=("nat" "sgx" "sgx-asan" "sgx-mpx" "sgxbounds-optall")
declare -a inputsarr=("memaslap-default")

#========================== EXPERIMENT SCRIPT =================================#
echo "===== Results for memcached benchmark ====="

# remove old logs
rm -f $resfile


MUSLVAL="MUSL_VERSION=1 MUSL_ETHREADS=4 MUSL_STHREADS=30"
MPXVAL="CHKP_RT_BNDPRESERVE=0 CHKP_RT_MODE=stop CHKP_RT_VERBOSE=1 CHKP_RT_PRINT_SUMMARY=1"
ASANVAL="ASAN_OPTIONS=verbosity=1:print_summary=true:"

for times in $(seq 1 $maxtries); do
  for in in "${inputsarr[@]}"; do
    for type in "${typesarr[@]}"; do
	  for clients in "${clientsarr[@]}"; do
		    addr="$ip"
		    sleep 1
		    # start server
		    echo "--- Running ${bm} type: ${type} clients: ${clients} input: ${in} ---" | tee -a ${resfile}
		    ${ssh} pkill -9 memcached > /dev/null || true

            ${ssh} $MUSLVAL $MPXVAL $ASANVAL ${remotedir}/${type}/bin/memcached -t 8 & 

		    # client run phase
		    sleep 20
            ~/bin/libmemcached-1.0.18/clients/memaslap -s ${sshhost}:11211 -t ${duration}s -c ${clients} -T 16 -S ${duration}s | tee -a ${resfile}

		    # kill server -- *it segfaults on killing, this is ok*
		    sleep 1
		    ${ssh} pkill -9 memcached  > /dev/null || true
      done # clients
    done # types
  done # times
done # inputs


