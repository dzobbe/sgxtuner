# sgxmusl-tuner (Work In Progress)
A tool for automatic tuning of SGX Musl library parameters written in Rust. The application searches for the best configuration parameters using the Simulated Annealing algorithm (https://en.wikipedia.org/wiki/Simulated_annealing), a stochastic process for iterated local search.

## Requirements
Of course, you will need Rust installed. If you haven't already, get it here: [rust-lang.org](https://www.rust-lang.org). Also you need Cargo [Cargo](https://crates.io) to easily compile. The rustc compiler version required is the 1.15.0-nightly.


## Usage

1. Clone the [source] with `git`:

   ```sh
   $ git clone https://github.com/dzobbe/sgxmusl-tuner.git
   $ cd sgxmusl-tuner
   ```
2. Build

     ```sh
    $ sudo cargo build
    ```
3. Configure the SGX-MUSL parameters in the `params.conf` file. The syntax is the following:

   ```
   [Param_name]:[min_value,max_value,step]:[initial_value]
   ```
4. Run the tuner by passing through the command line: 
   * The path to the binary of the Target application to test
   * The Target arguments
   * The path to the binary of the Benchmark application
   * The Benchmark arguments
   * The parameters needed by the Simulated Annealing algorithm
   **Note 1** -> The Benchmark MUST be started `localhost:12349` that is the address on which the MeterProxy listens
   **Note 2** -> The address and port of the target application MUST be specified in its arguments. The Tuner application, in fact, automatically searches in the Target arguments for the first occurrences of -p/--port and -l/-h/--address/--host. 
   
   ```sh
   $ Usage:   sgxmusl-tuner [-t] --targ=<targetPath> [--args2targ=<args>] [-b] \
   --bench=<benchmarkPath> [--args2bench=<args>] [-ms]                         \
   --maxSteps=<maxSteps> [-t] --maxTemp=<maxTemperature> [-mt]                 \
   --minTemp=<minTemperature> [-at] --maxAtt=<maxAttempts> [-ac]               \
   --maxAcc=   <maxAccepts> [-rj] --maxRej=<maxRejects>                        \
   --energy=<energy> --cooling=<cooling>
   
  Options:
    -t,    --targ=<args>         #Target Path
    --args2targ=<args>           #Arguments for Target
    -b,    --bench=<args>        #Benchmark Path
    --args2bench=<args>          #Arguments for Benchmark
    -ms,   --maxSteps=<args>     #Max Steps of Annealing
    -tp,   --maxTemp=<args>      #Max Temperature
    -mt,   --minTemp=<args>      #Min Temperature
    -at,   --maxAtt=<args>       #Max Attemtps
    -ac,   --maxAcc=<args>       #Max Accepts
    -rj,   --maxRej=<args>       #Max Rejects  
    -e,	   --energy=<args>      #Energy to eval (latency or throughput)
    -c,    --cooling=<args>      #Cooling Schedule (linear, exponential, adaptive)
   ```
   

## Example
In this example we run the `sgxmusl-tuner` on memcached using as a benchmark the [mcperf](https://github.com/twitter/twemperf) tool.

1. Configure the SGX-MUSL parameters in the `params.conf` file. 

   ```
   MUSL_ETHREADS:[6,10,1]:6
   MUSL_STHREADS:[3,8,1]:5
   MUSL_ESPINS:[400,600,10]:500
   MUSL_SSPINS:[80,120,10]:100
   MUSL_ESLEEP:[14000,18000,1000]:16000
   MUSL_SSLEEP:[3000,4000,100]:4000
   ```

2.  Run the tuner. Yes I know, the arguments for the `sgxmusl-tuner` are too much. An acquisition through an .xml configuration file is one of the TODOs. Anyway, for the moment the `start-tuning.sh` script can help.

   ```sh
   $ target/debug/sgxmusl-tuner --targ=$MEMCACHED_HOME/bin/memcached \
    --args2targ="-l 127.0.0.1 -p 12347" \
    --bench=$MCPERF_HOME/bin/mcperf \
    --args2bench="-p 12349 --linger=0 --timeout=5 --conn-rate=1000 --call-rate=1000 --num-calls=10 --num-conns=1000 --sizes=u1,16" \
    --maxSteps=10000 --maxTemp=1000 --minTemp=2 --maxAtt=100 --maxAcc=100 --maxRej=500 \
    --energy=throughput \
    --cooling=exponential
   ```

## TODOs and Open Issues
The following issues and TODOs need to be solved:
* Currently there are issues with the measurement of the latency enforced by the `MeterProxy`. Therefore the only energy that can be evaluated at the moment is the throughput of responses.
* The `adaptive` cooling schedule still need to be developed.
* The provision of the metrics to `InfluxDB` and then to the `Chronograf`need to be developed
* A parallel version of the algorithm is under construction
* An acquisition of arguments through a .xml file
