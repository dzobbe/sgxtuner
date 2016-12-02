# annealing-tuner (Work In Progress)
A tool for automatic tuning of client-server applications parameters. The application searches for the best configuration parameters using the Simulated Annealing algorithm (https://en.wikipedia.org/wiki/Simulated_annealing), a stochastic process for iterated local search.

## Requirements
Of course, you will need Rust installed. If you haven't already, get it here: [rust-lang.org](https://www.rust-lang.org). Also you need [Cargo](https://crates.io) to easily compile. The rustc compiler version required is the 1.15.0-nightly.


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
3. Configure the parameters in the `params.conf` file. The syntax is the following:

   ```
   [Param_name]:[min_value,max_value,step]:[initial_value]
   ```
4. Run the tuner by passing through the command line: 
   * The path to the binary of the Target application to test
   * The Target arguments
   * The path to the binary of the Benchmark application
   * The Benchmark arguments
   * The parameters needed by the tool to carry out the Simulated Annealing:
         * maxSteps : the number of steps to carry out the annealing process. Higher is this number, more accurate will be the final result at the cost of execution time. 
         * numIter  : The number of iterations to spend on each stage of exploration to get average measurements. 
         * maxTemp  : The starting temperature of the annealing process. Usually this value is set to a value that allows to choose the 98% of the moves. 
         * minTemp  : The final temperature of the annealing process.
         * energy   : The type of energy to measure (Throughput or Latency)
         * cooling  : The cooling function of the temperature (Exponential, Linear, Adaptive)
   
   ⚠️ **Note 1** - The Benchmark MUST be started on `localhost:12349` that is the address on which the `MeterProxy` listens
   
   ⚠️ **Note 2** - The address and port of the target application MUST be specified in its arguments. The Tuner application, in fact, automatically searches in the Target arguments for the first occurrences of `-p/--port` and `-l/-h/--address/--host`. 
   
   ```sh
   $ Usage:   annealing-tuner [-t] --targ=<targetPath> [--args2targ=<args>] [-b] \
   --bench=<benchmarkPath> [--args2bench=<args>] [-ms] --maxSteps=<maxSteps>   \
   [-ni] --numIter=<numIter>        [-tp] --maxTemp=<maxTemperature>           \
   [-mt] --minTemp=<minTemperature> [-e] --energy=<energy>                     \
   [-c] --cooling=<cooling>
   
  Options:
    -t,    --targ=<args>         #Target Path
    --args2targ=<args>           #Arguments for Target
    -b,    --bench=<args>        #Benchmark Path
    --args2bench=<args>          #Arguments for Benchmark
    -ms,   --maxSteps=<args>     #Max Steps of Annealing
    -ni,   --numIter=<args>      #Number of Iterations for each stage of exploration
    -tp,   --maxTemp=<args>      #Max Temperature
    -mt,   --minTemp=<args>      #Min Temperature 
    -e,	   --energy=<args>      #Energy to eval (latency or throughput)
    -c,    --cooling=<args>      #Cooling Schedule (linear, exponential, adaptive)
   ```
   

## Example
In this example we run the `annealing-tuner` on memcached using as a benchmark the [mcperf](https://github.com/twitter/twemperf) tool.

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
    --maxSteps=10000 --numIter=5 --maxTemp=1000 --minTemp=2  \
    --energy=throughput \
    --cooling=exponential
   ```

## TODOs and Open Issues
The following issues and TODOs need to be solved:
* The `adaptive` cooling schedule still need to be developed.
* The provision of the metrics to `InfluxDB` and then to the `Chronograf`need to be developed
* A parallel version of the algorithm is under construction
* An acquisition of arguments through a .xml file
