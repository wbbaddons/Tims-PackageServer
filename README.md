Tim’s PackageServer
===================

Tim’s PackageServer is a lightweight, node.js based package server for [WoltLab Community Framework](https://github.com/WoltLab/WCF).

How to use?
-----------

1. Install the dependencies:

    ```sh
    $ npm install
    ```

2. Start the package server:

    ```sh
    $ npm start
    ```
    
3. Create a folder named the same as the package identifier in packages:

    ```
    $ mkdir packages/com.example.wcf.package
    ```

4. Drop the built archive named the same as the version number into the package folder:

    ```sh
    $ cp com.example.wcf.package.tar packages/com.example.wcf.package/1.0.0.tar
    ```

5. Open the package server in the browser, your package should appear.

auth.json
---------

The grammar for valid version checks can be found in versionComparator.jison.

Configuration
-------------

Tim’s PackageServer is configured using [rc](https://www.npmjs.com/package/rc). The
following configuration options are available:

| Name             | Default Value            | Explanation                                        |
|------------------|--------------------------|----------------------------------------------------|
| port             | 9001                     | The port to bind to                                |
| ip               | '0.0.0.0'                | The ip to bind to                                  |
| packageFolder    | __dirname + "/packages/" | The folder the packages are searched in            |
| enableStatistics | true                     | Whether to enable download counters                |
| enableHash       | true                     | Whether to provide a SHA-256 hash of every version |
| deterministic    | false                    | Hides per request statistics                       |
| ssl              | false                    | Whether to advertise SSL support                   |

Debugging
---------

Tim’s PackageServer uses the [debug](https://github.com/visionmedia/debug) package to output
debug information. You can access it by setting the environment variable `DEBUG` to `PackageServer:*`.

License
-------

For licensing information refer to the LICENSE file in this folder.
