import os
import shutil

from setuptools import setup, find_packages

version = os.environ.get("PACKAGE_VERSION")
environment = os.environ.get("ENVIRONMENT")
base_package_name = "retake"
package_name = (
    f"{base_package_name}_{environment}"
    if environment in ["dev", "staging"]
    else base_package_name
)

package_dir = dict()
package_dir[package_name] = "interface"

if os.path.exists("dist"):
    shutil.rmtree("dist")

setup(
    version=version,
    name=package_name,
    package_dir=package_dir,
    packages=find_packages(),
    install_requires=["pydantic", "psycopg2-binary", "openai"],
)
