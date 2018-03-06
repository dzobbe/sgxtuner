# SGXTuner
SGXTuner is a distributed tuning system that uses stochastic optimization to enhance the performance of applications hardened with SGX. More precisely, the tuner leverages a self-implemented parallel [Simulated Annealing](https://en.wikipedia.org/wiki/Simulated_annealing) algorithm to find a near-optimal parameters configuration of SGX-enabled libc libraries, which are the basis of current approaches for executing unmodified legacy applications on SGX. Different SA algorithm are implemented, these are:

   * SEQSA  - This is the standard simulated annealing solver, which searches for the best solution in a sequential way.
   * SPISA - This a parallelized version of the simulated annealing in which the different worker machines explore in parallel a specific set of neighborhoods composed by indipendent configurations and periodically exchange information.
   * MIPS - This additional parallelized version of the solver starts from different initial parameter configurations and executes multiple indipendent workers, which don't need to exchange information except for the final comparison of worker results.
   * PRSA - This last parallel version, instead, applies the "Parallel Recombinative Simulated Annealing" algorithm, which is combination of the Genetic Crossover algorithm and Simulated Annealing 

A 6-parameters tuning activity has been performed for a particular extended libc library, namely sgx-musl, which underlies the widely accepted SGX-secured containers, i.e., [SCONE](https://www.usenix.org/system/files/conference/osdi16/osdi16-arnautov.pdf)

## Requirements
Of course, you will need Rust installed. If you haven't already, get it here: [rust-lang.org](https://www.rust-lang.org). Also you need [Cargo](https://crates.io) to easily compile. The rustc compiler version required is the 1.15.0-nightly.

You also need [docker](https://github.com/docker) and [docker-compose](https://github.com/docker/compose)

## Usage

1. Clone the [source] with `git`:

   ```sh
   $ git clone https://github.com/dzobbe/sgxtuner.git
   $ cd sgxtuner/
   ```
2. Build

    ```sh
    $ docker-compose build
    ```
3. Configure the tuner through the `conf.xml` file. The configuration consists of four main sections:

    * Target - This allows to define the Target node (or nodes) that will run the sgx-musl compiled target application. The set of parameters to tune are:
        * Execution - Means the type of execution of the application, if local or remote
        * Host - If remote, the host address where the application will be started must be specified
        * User - If remote, the host username must be specified
        * Bin - The binary name of the target application (e.g. memcached)
        * Path - The path to the binary of the target application
        * Args - The arguments to pass in input to the target application
        * Address - The address on which the target application will listen
        * Port - The port on which the target application will listen
    
    * Bench - Equal to the Target configuration section except for:
        ** Name - Meaning the name of the benchmark which is going to be launched. Currently supported: wrk(apache), ycsb(redis), memaslap(memcached). This is needed to select the correct output parser. Its source code is in `src/energy_eval/output_parser.rs`. It is not a big deal to develop a specific parser function. However, it is needed a way for easily adapt other benchmarks

        
    * Annealing - Useful to configure main relevant parameters of the simulated annealing algorithm
        * Max Step - The maximum number of steps after which the tuner must stop if it wasn't able to converge 
        * Num Iter - The number of runs to perform for each sgx-musl parameter configuration  
        * Min Temp - The minimum temperature that the simulated annealing can reach 
        * Max Temp - The maximum temperature at which start the exploration. If Min & Max Temp are left empty, the tuner automatically define them. Have a look to the paper for more information.
        * Energy - The energy type, i.e., `throughput` (maximization job) or `latency` (minimization job)
        * Cooling - The cooling strategy for the temperature, i.e., `exponential`, `linear`, or `basic_exp_cooling`
        * Problem - (Don't care about this, it was needed for test purposes. Leave it as `default`)
        * Version - The version of simulated annealig to run, i.e., `seqsa`, `spisa`, `mir`, or `prsa`
        * Workers - The number of workers (as many as the number of launched Targets)
        
    * Musl-Params - Needed to configure the 6 sgx-musl parameters exploration space. More precisely, the user needs to define:
        * Name - The sgx-musl parameter name that will be used to declare the correspondent environment variable
        * Default - The initial value used
        * Minimum - The minimum value that can be assumed
        * Maximum - The maximum value that can be assumed
        * Step - The step of variation between the minimum and maximum
    
  
4. Run the tuner by launching from sgx-musl-annealing-tuner/Tuner-Code:

   ```sh
   ./target/debug/annealing-tuner
   ```

5. The tuner logs the stochastic exploration in the `results.csv` file. Such a file will include as many entries as the number of annealing steps conducted. Each entry provides information on the best parameters and energy till that point, and also the measurements for the specific step. Furthermore, the line of the CSV file includes also the results of the evaluation for that step with a specific configuration of parameters.

## Example
In this example we run the `sgx-musl-annealing-tuner` to launch a parallel job consisting of two `Memcached` Targets and a `Memaslap` Benchmark.

1. Configure the Target 

   ```xml
        <m1>
            <execution>remote</execution>
            <host>10.3.1.1:22</host>
            <user>giovanni</user>
            <bin>memcached</bin>
            <path>/home/giovanni/memcached/sgx-memcached/</path>
            <args>-l 10.3.1.1 -p 14200 -t 8</args>
            <address>10.3.1.1</address>
            <port>14200</port>
        </m1>
        <m2>
            <execution>remote</execution>
            <host>10.3.1.2:22</host>
            <user>giovanni</user>
            <bin>memcached</bin>
            <path>/home/giovanni/memcached/sgx-memcached/</path>
            <args>-l 10.3.1.2 -p 14200 -t 8</args>
            <address>10.3.1.2</address>
            <port>14200</port>
        </m2>
   
   ```
2. Configure the Benchmark  

   ```xml
	    <name>memaslap</name>
	    <execution>local</execution>
	    <host></host>
	    <user></user>
	    <bin>memaslap</bin>
	    <path>/home/giovanni/bin/libmemcached-1.0.18/clients/</path>
	    <args>-s 10.3.1.2:14200 -t 20s -c 512 -T 16 -S 20s</args>
	    <address>10.3.1.2</address>
	    <port>14200</port>   
   ```
3. Configure the Simulated Annealing algorithm

   ```xml
        <max_step>10000</max_step>
        <num_iter>1</num_iter>
        <min_temp></min_temp>
        <max_temp></max_temp>
        <energy>throughput</energy>
        <cooling>exponential</cooling>
        <problem>default</problem>
        <version>spisa</version>
        <workers>2</workers>
   ```
4. Configure the `sgx-musl` parameters

   ```xml
       <musl-params>
            <sthreads>
                    <name>MUSL_STHREADS</name>
                    <default>32</default>
                    <minimum>20</minimum>
                    <maximum>70</maximum>
                    <step>5</step>
            </sthreads>
            <ethreads>
                    <name>MUSL_ETHREADS</name>
                    <default>4</default>
                    <minimum>1</minimum>
                    <maximum>10</maximum>
                    <step>1</step>
            </ethreads>
            <ssleep>
                    <name>MUSL_SSLEEP</name>
                    <default>3400</default>
                    <minimum>2800</minimum>
                    <maximum>4200</maximum>
                    <step>100</step>
            </ssleep>
            <esleep>
                    <name>MUSL_ESLEEP</name>
                    <default>15000</default>
                    <minimum>12000</minimum>
                    <maximum>20000</maximum>
                    <step>1000</step>
            </esleep>
            <sspins>
                    <name>MUSL_SSPINS</name>
                    <default>80</default>
                    <minimum>50</minimum>
                    <maximum>140</maximum>
                    <step>10</step>
            </sspins>
            <espins>
                    <name>MUSL_ESPINS</name>
                    <default>4800</default>
                    <minimum>3400</minimum>
                    <maximum>6400</maximum>
                    <step>200</step>
            </espins>
    </musl-params>
   
   ```
2.  Run the tuner. 

   ```sh
   $ docker-compose up 
   ```

## Open Issues and Future Work
The tuner initially made use of a meter proxy which allowed to run any target and benchmark without caring of the output (e.g. the throughput) collection. This solution was abandoned since the impact on the measurements was not negligibile.

## License

MIT © [Giovanni Mazzeo](https://github.com/dzobbe)
