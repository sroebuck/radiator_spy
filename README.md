# radiator_spy

This code reads the [FHT Protocol](https://sourceforge.net/p/opentrv/wiki/FHT%20Protocol/) messages from a range of radiator controllers branded by Conrad and some other companies using a cc1101 chip connected to a Raspberry PI.

The codebase started from the [CC1101 based IKEA Sparsnäs decoder](https://github.com/dsvensson/sparsnas-rs) and uses a [forked augmented version of his cc1101 controller library](https://github.com/sroebuck/cc1101).  I hope to tidy up my changes to the library and offer them back when I get a chance.

At the present time this is very much a work in progress.  It now successfully reads the messages from the radiator controllers but there are lots of debuging `println`s and all the other ugliness that accompanies a long drawn out trial and error process as I tried to figure out how to control the cc1101.

Hopefully this will get tidied up too when I have a chance.  My hope is to turn this into a useful library that will allow the radiator controllers to be monitored (to monitor energy consumption and identify ways of improving it).  I also hope to add code to send signals to the radiators so that it becomes possible to override the controller signals in various ways.


Feel free to copy / modify / adapt the code.
