import os
import shutil
import subprocess
import toml

from setuptools import setup, find_packages


def build():
    # Load and parse pyproject.toml
    pyproject = toml.load("pyproject.toml")

    # Extract version from pyproject.toml
    version = pyproject["tool"]["poetry"]["version"]

    environment = os.environ.get("ENVIRONMENT")
    base_package_name = "retake"
    entry_point_folder = "sdk"

    package_name = (
        f"{base_package_name}_{environment}"
        if environment in ["dev", "staging"]
        else base_package_name
    )

    package_dir = dict()
    package_dir[package_name] = entry_point_folder
    packages = [package_name, "core"]

    if os.path.exists("dist"):
        shutil.rmtree("dist")

    if os.path.exists("build"):
        shutil.rmtree("build")

    setup(
        version=version,
        name=package_name,
        package_dir=package_dir,
        packages=packages,
        install_requires=["pydantic", "psycopg2-binary", "openai", "elasticsearch"],
    )


def install():
    # Find wheel file
    dist_dir = os.path.join(os.getcwd(), "dist")
    wheel_file = next((f for f in os.listdir(dist_dir) if f.endswith(".whl")), None)

    if wheel_file is None:
        raise FileNotFoundError("No .whl file found in dist directory")

    # Install wheel file
    subprocess.run(
        ["pip", "install", "--force-reinstall", os.path.join(dist_dir, wheel_file)],
        check=True,
    )


def publish():
    # Check if twine is installed
    try:
        subprocess.run(["twine", "--version"], check=True, stdout=subprocess.DEVNULL)
    except subprocess.CalledProcessError:
        print("Twine is not installed. Please install it with 'pip install twine'.")
        return

    # PUblis to PyPI
    dist_dir = os.path.join(os.getcwd(), "dist")
    subprocess.run(["twine", "upload", os.path.join(dist_dir, "*")], check=True)
