from setuptools import setup, find_packages
import os

version = os.environ.get("PACKAGE_VERSION")
environment = os.environ.get("ENVIRONMENT")
package_name = f"retake-connect-{a}" if a in ["dev", "staging"] else "retake-connect"

package_dir = dict()
package_dir[package_name] = "../interface"

setup(
    version=version,
    name=package_name,
    package_dir=package_dir,
    packages=[package_name],
    install_requires=["pydantic==2.0", "psycopg2-binary==2.9.6"],
)
