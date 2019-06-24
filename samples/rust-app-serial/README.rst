Build
=====

.. code-block:: console

    mkdir -p build-x86 && cd build-x86
    cmake -GNinja -DBOARD=qemu_x86 ..
    ninja

Run
===

.. code-block:: console
    sh ../run.sh

A `run.sh` script has been provided which adds the following line to the qemu
command:

.. code-block:: console
    -serial tcp:localhost:4444,server,nowait

You can then connect to localhost:4444 using nc to simulate the other end of
the serial connection.

Of course, you can change the -serial argument to any other option that qemu
supports.
