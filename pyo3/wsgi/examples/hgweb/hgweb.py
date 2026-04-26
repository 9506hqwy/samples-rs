from os import path
from mercurial import demandimport
demandimport.enable()

from mercurial.hgweb.hgwebdir_mod import hgwebdir

conf_path = path.join(path.dirname(__file__), "hgweb.config")
application = hgwebdir(conf_path.encode())
